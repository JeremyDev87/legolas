import { promises as fs } from "node:fs";
import path from "node:path";

const SOURCE_FILE_PATTERN = /\.(c|m)?(j|t)sx?$|\.vue$|\.svelte$/;
const IGNORED_DIRECTORIES = new Set([
  ".git",
  "node_modules",
  "dist",
  "build",
  ".next",
  ".turbo",
  "coverage",
  ".output",
  "test",
  "tests",
  "__tests__"
]);

export async function collectSourceFiles(projectRoot) {
  const files = [];
  await walk(projectRoot, files);
  return files;
}

export async function scanImports(projectRoot, sourceFiles) {
  const byPackage = new Map();
  const treeShakingObservations = [];
  let dynamicImportCount = 0;

  for (const absolutePath of sourceFiles) {
    const contents = await fs.readFile(absolutePath, "utf8");
    const relativePath = path.relative(projectRoot, absolutePath);
    const scannableContents = getScannableContents(absolutePath, contents);
    const { imports, treeShakingHints } = scanSourceFile(scannableContents, {
      jsxTextGuard: supportsJsxTextGuard(absolutePath)
    });

    for (const entry of imports) {
      const packageName = normalizePackageName(entry.specifier);
      if (!packageName) {
        continue;
      }

      if (!byPackage.has(packageName)) {
        byPackage.set(packageName, {
          name: packageName,
          files: new Set(),
          staticFiles: new Set(),
          dynamicFiles: new Set()
        });
      }

      const record = byPackage.get(packageName);
      record.files.add(relativePath);
      if (entry.kind === "dynamic") {
        record.dynamicFiles.add(relativePath);
        dynamicImportCount += 1;
      } else {
        record.staticFiles.add(relativePath);
      }
    }

    treeShakingObservations.push(...treeShakingHints.map((hint) => ({
      ...hint,
      files: new Set([relativePath])
    })));
  }

  return {
    byPackage,
    importedPackages: [...byPackage.values()].sort((left, right) => left.name.localeCompare(right.name)),
    dynamicImportCount,
    treeShakingWarnings: mergeTreeShakingWarnings(treeShakingObservations)
  };
}

async function walk(currentPath, files) {
  const entries = await fs.readdir(currentPath, { withFileTypes: true });

  for (const entry of entries) {
    const absolutePath = path.join(currentPath, entry.name);

    if (entry.isDirectory()) {
      if (IGNORED_DIRECTORIES.has(entry.name)) {
        continue;
      }
      await walk(absolutePath, files);
      continue;
    }

    if (SOURCE_FILE_PATTERN.test(entry.name)) {
      files.push(absolutePath);
    }
  }
}

function scanSourceFile(contents, options = {}) {
  const imports = [];
  const treeShakingHints = [];
  let index = 0;

  while (index < contents.length) {
    const character = contents[index];

    if (character === "/" && contents[index + 1] === "/") {
      index = skipLineComment(contents, index);
      continue;
    }

    if (character === "/" && contents[index + 1] === "*") {
      index = skipBlockComment(contents, index);
      continue;
    }

    if (character === "'" || character === "\"") {
      index = skipQuotedString(contents, index, character);
      continue;
    }

    if (character === "`") {
      index = skipTemplateString(contents, index);
      continue;
    }

    if (!isIdentifierStart(character)) {
      index += 1;
      continue;
    }

    const token = readIdentifier(contents, index);

    if (options.jsxTextGuard && isInsideLikelyJsxText(contents, index)) {
      index += token.length;
      continue;
    }

    if (token === "import") {
      const parsed = tryParseImport(contents, index);
      if (parsed) {
        if (parsed.importEntry) {
          imports.push(parsed.importEntry);
        }
        if (parsed.treeShakingHint) {
          treeShakingHints.push(parsed.treeShakingHint);
        }
        index = parsed.nextIndex;
        continue;
      }
    }

    if (token === "export") {
      const parsed = tryParseExportFrom(contents, index);
      if (parsed) {
        if (parsed.importEntry) {
          imports.push(parsed.importEntry);
        }
        index = parsed.nextIndex;
        continue;
      }
    }

    if (token === "require") {
      const parsed = tryParseRequire(contents, index);
      if (parsed) {
        imports.push(parsed.importEntry);
        index = parsed.nextIndex;
        continue;
      }
    }

    index += token.length;
  }

  return { imports, treeShakingHints };
}

function normalizePackageName(specifier) {
  if (
    !specifier ||
    specifier.startsWith("node:") ||
    specifier.startsWith(".") ||
    specifier.startsWith("/") ||
    specifier.startsWith("~/") ||
    specifier.startsWith("@/") ||
    specifier.startsWith("#") ||
    specifier.startsWith("virtual:")
  ) {
    return null;
  }

  if (specifier.startsWith("@")) {
    const [scope, name] = specifier.split("/");
    return scope && name ? `${scope}/${name}` : null;
  }

  return specifier.split("/")[0];
}

function mergeTreeShakingWarnings(warnings) {
  const merged = new Map();

  for (const warning of warnings) {
    const mapKey = `${warning.key}:${warning.packageName}`;
    if (!merged.has(mapKey)) {
      merged.set(mapKey, warning);
      continue;
    }

    const existing = merged.get(mapKey);
    for (const file of warning.files) {
      existing.files.add(file);
    }
    existing.estimatedKb = Math.max(existing.estimatedKb, warning.estimatedKb);
  }

  return [...merged.values()];
}

function getScannableContents(filePath, contents) {
  const extension = path.extname(filePath);

  if (extension === ".vue" || extension === ".svelte") {
    return extractScriptBlocks(contents);
  }

  return contents;
}

function extractScriptBlocks(contents) {
  const scriptBlocks = [];
  const scriptBlockPattern = /<script\b[^>]*>([\s\S]*?)<\/script>/gi;

  for (const match of contents.matchAll(scriptBlockPattern)) {
    scriptBlocks.push(match[1]);
  }

  return scriptBlocks.join("\n");
}

function supportsJsxTextGuard(filePath) {
  return [".js", ".jsx", ".ts", ".tsx", ".mjs", ".cjs"].includes(path.extname(filePath));
}

function tryParseImport(contents, startIndex) {
  if (!hasTokenBoundary(contents, startIndex, "import")) {
    return null;
  }

  let index = skipTrivia(contents, startIndex + "import".length);
  const character = contents[index];

  if (character === "(") {
    const parsedArgument = parseQuotedArgument(contents, index);
    if (!parsedArgument) {
      return null;
    }

    return {
      importEntry: {
        kind: "dynamic",
        specifier: parsedArgument.specifier
      },
      nextIndex: parsedArgument.nextIndex
    };
  }

  if (character === "'" || character === "\"") {
    const parsedString = readStringLiteral(contents, index);
    if (!parsedString) {
      return null;
    }

    return {
      importEntry: {
        kind: "static",
        specifier: parsedString.value
      },
      nextIndex: parsedString.nextIndex
    };
  }

  if (character === ".") {
    return null;
  }

  const fromIndex = findKeyword(contents, index, "from");
  if (fromIndex === -1) {
    return null;
  }

  const clause = contents.slice(index, fromIndex).trim();
  const parsedSpecifier = readStringLiteral(contents, skipTrivia(contents, fromIndex + "from".length));
  if (!parsedSpecifier) {
    return null;
  }

  if (isTypeOnlyClause(clause)) {
    return {
      nextIndex: parsedSpecifier.nextIndex
    };
  }

  return {
    importEntry: {
      kind: "static",
      specifier: parsedSpecifier.value
    },
    treeShakingHint: buildTreeShakingHint(parsedSpecifier.value, clause),
    nextIndex: parsedSpecifier.nextIndex
  };
}

function tryParseExportFrom(contents, startIndex) {
  if (!hasTokenBoundary(contents, startIndex, "export")) {
    return null;
  }

  const searchStart = skipTrivia(contents, startIndex + "export".length);
  const fromIndex = findKeyword(contents, searchStart, "from");
  if (fromIndex === -1) {
    return null;
  }

  const parsedSpecifier = readStringLiteral(contents, skipTrivia(contents, fromIndex + "from".length));
  if (!parsedSpecifier) {
    return null;
  }

  const clause = contents.slice(searchStart, fromIndex).trim();
  if (isTypeOnlyClause(clause)) {
    return {
      nextIndex: parsedSpecifier.nextIndex
    };
  }

  return {
    importEntry: {
      kind: "static",
      specifier: parsedSpecifier.value
    },
    nextIndex: parsedSpecifier.nextIndex
  };
}

function tryParseRequire(contents, startIndex) {
  if (!hasTokenBoundary(contents, startIndex, "require")) {
    return null;
  }

  const parsedArgument = parseQuotedArgument(contents, skipTrivia(contents, startIndex + "require".length));
  if (!parsedArgument) {
    return null;
  }

  return {
    importEntry: {
      kind: "static",
      specifier: parsedArgument.specifier
    },
    nextIndex: parsedArgument.nextIndex
  };
}

function buildTreeShakingHint(specifier, clause) {
  const normalizedClause = clause.replace(/\s+/g, " ").trim();

  if (/^\*\s+as\s+\w+$/.test(normalizedClause) && isNamespaceSensitivePackage(specifier)) {
    return {
      key: "namespace-ui-import",
      packageName: specifier,
      message: "Namespace imports pull large symbol sets into a single module graph.",
      recommendation: "Import only the symbols you need from direct subpaths.",
      estimatedKb: 35
    };
  }

  if (specifier === "lodash" && normalizedClause.length > 0) {
    return {
      key: "lodash-root-import",
      packageName: "lodash",
      message: "Root lodash imports often keep more code than expected in client bundles.",
      recommendation: "Prefer per-method imports or lodash-es.",
      estimatedKb: 26
    };
  }

  if (specifier === "react-icons") {
    return {
      key: "react-icons-root-import",
      packageName: "react-icons",
      message: "Root react-icons imports can make tree shaking unreliable.",
      recommendation: "Import from the specific icon pack path instead.",
      estimatedKb: 22
    };
  }

  return null;
}

function isTypeOnlyClause(clause) {
  const normalizedClause = clause.replace(/\s+/g, " ").trim();

  if (normalizedClause.length === 0) {
    return false;
  }

  if (normalizedClause.startsWith("type ")) {
    return true;
  }

  if (!normalizedClause.startsWith("{") || !normalizedClause.endsWith("}")) {
    return false;
  }

  const specifiers = normalizedClause
    .slice(1, -1)
    .split(",")
    .map((specifier) => specifier.trim())
    .filter(Boolean);

  return specifiers.length > 0 && specifiers.every((specifier) => specifier.startsWith("type "));
}

function isNamespaceSensitivePackage(specifier) {
  return specifier === "lodash" || specifier === "lucide-react" || specifier === "@mui/icons-material";
}

function parseQuotedArgument(contents, startIndex) {
  let index = skipTrivia(contents, startIndex);
  if (contents[index] !== "(") {
    return null;
  }

  index = skipTrivia(contents, index + 1);
  const parsedString = readStringLiteral(contents, index);
  if (!parsedString) {
    return null;
  }

  return {
    specifier: parsedString.value,
    nextIndex: parsedString.nextIndex
  };
}

function readStringLiteral(contents, startIndex) {
  const quote = contents[startIndex];
  if (quote !== "'" && quote !== "\"") {
    return null;
  }

  let value = "";
  let index = startIndex + 1;

  while (index < contents.length) {
    const character = contents[index];

    if (character === "\\") {
      const nextCharacter = contents[index + 1];
      if (nextCharacter === undefined) {
        return null;
      }
      value += nextCharacter;
      index += 2;
      continue;
    }

    if (character === quote) {
      return {
        value,
        nextIndex: index + 1
      };
    }

    value += character;
    index += 1;
  }

  return null;
}

function findKeyword(contents, startIndex, keyword) {
  let index = startIndex;
  let depth = 0;

  while (index < contents.length) {
    const character = contents[index];

    if (character === "/" && contents[index + 1] === "/") {
      index = skipLineComment(contents, index);
      continue;
    }

    if (character === "/" && contents[index + 1] === "*") {
      index = skipBlockComment(contents, index);
      continue;
    }

    if (character === "'" || character === "\"") {
      index = skipQuotedString(contents, index, character);
      continue;
    }

    if (character === "`") {
      index = skipTemplateString(contents, index);
      continue;
    }

    if (character === "{" || character === "(" || character === "[") {
      depth += 1;
      index += 1;
      continue;
    }

    if (character === "}" || character === ")" || character === "]") {
      depth = Math.max(0, depth - 1);
      index += 1;
      continue;
    }

    if (depth === 0 && isKeywordAt(contents, index, keyword)) {
      return index;
    }

    if (depth === 0 && character === ";") {
      return -1;
    }

    index += 1;
  }

  return -1;
}

function skipTrivia(contents, startIndex) {
  let index = startIndex;

  while (index < contents.length) {
    const character = contents[index];

    if (/\s/.test(character)) {
      index += 1;
      continue;
    }

    if (character === "/" && contents[index + 1] === "/") {
      index = skipLineComment(contents, index);
      continue;
    }

    if (character === "/" && contents[index + 1] === "*") {
      index = skipBlockComment(contents, index);
      continue;
    }

    return index;
  }

  return index;
}

function skipLineComment(contents, startIndex) {
  let index = startIndex + 2;
  while (index < contents.length && contents[index] !== "\n") {
    index += 1;
  }
  return index;
}

function skipBlockComment(contents, startIndex) {
  let index = startIndex + 2;
  while (index < contents.length - 1) {
    if (contents[index] === "*" && contents[index + 1] === "/") {
      return index + 2;
    }
    index += 1;
  }
  return contents.length;
}

function skipQuotedString(contents, startIndex, quote) {
  let index = startIndex + 1;

  while (index < contents.length) {
    const character = contents[index];
    if (character === "\\") {
      index += 2;
      continue;
    }
    if (character === quote) {
      return index + 1;
    }
    index += 1;
  }

  return contents.length;
}

function skipTemplateString(contents, startIndex) {
  let index = startIndex + 1;

  while (index < contents.length) {
    const character = contents[index];

    if (character === "\\") {
      index += 2;
      continue;
    }

    if (character === "`") {
      return index + 1;
    }

    if (character === "$" && contents[index + 1] === "{") {
      index = skipBalancedExpression(contents, index + 2);
      continue;
    }

    index += 1;
  }

  return contents.length;
}

function skipBalancedExpression(contents, startIndex) {
  const stack = ["}"];
  let index = startIndex;

  while (index < contents.length && stack.length > 0) {
    const character = contents[index];

    if (character === "/" && contents[index + 1] === "/") {
      index = skipLineComment(contents, index);
      continue;
    }

    if (character === "/" && contents[index + 1] === "*") {
      index = skipBlockComment(contents, index);
      continue;
    }

    if (character === "'" || character === "\"") {
      index = skipQuotedString(contents, index, character);
      continue;
    }

    if (character === "`") {
      index = skipTemplateString(contents, index);
      continue;
    }

    if (character === "{" || character === "(" || character === "[") {
      stack.push(getClosingCharacter(character));
      index += 1;
      continue;
    }

    if (character === stack[stack.length - 1]) {
      stack.pop();
      index += 1;
      continue;
    }

    index += 1;
  }

  return index;
}

function getClosingCharacter(openCharacter) {
  if (openCharacter === "{") {
    return "}";
  }

  if (openCharacter === "(") {
    return ")";
  }

  return "]";
}

function hasTokenBoundary(contents, startIndex, token) {
  const previousCharacter = contents[startIndex - 1];
  const nextCharacter = contents[startIndex + token.length];

  const validPrevious = previousCharacter === undefined || (!isIdentifierCharacter(previousCharacter) && previousCharacter !== ".");
  const validNext = nextCharacter === undefined || !isIdentifierCharacter(nextCharacter);

  return validPrevious && validNext;
}

function isKeywordAt(contents, startIndex, keyword) {
  return contents.startsWith(keyword, startIndex) && hasTokenBoundary(contents, startIndex, keyword);
}

function isInsideLikelyJsxText(contents, startIndex) {
  const previousNonWhitespaceIndex = findPreviousNonWhitespace(contents, startIndex - 1);
  if (previousNonWhitespaceIndex === -1 || contents[previousNonWhitespaceIndex] !== ">") {
    return false;
  }

  const previousTagStart = contents.lastIndexOf("<", previousNonWhitespaceIndex);
  if (previousTagStart === -1) {
    return false;
  }

  const previousTag = contents.slice(previousTagStart, previousNonWhitespaceIndex + 1);
  if (!isLikelyJsxTag(previousTag)) {
    return false;
  }

  let nextBoundaryIndex = startIndex;
  while (nextBoundaryIndex < contents.length) {
    const character = contents[nextBoundaryIndex];
    if (
      character === "<" ||
      character === "{" ||
      character === "}" ||
      character === ";" ||
      character === "\n" ||
      character === "\r"
    ) {
      break;
    }
    nextBoundaryIndex += 1;
  }

  if (contents[nextBoundaryIndex] !== "<") {
    return false;
  }

  const nextTagEnd = contents.indexOf(">", nextBoundaryIndex);
  if (nextTagEnd === -1) {
    return false;
  }

  const nextTag = contents.slice(nextBoundaryIndex, nextTagEnd + 1);
  return isLikelyJsxTag(nextTag);
}

function findPreviousNonWhitespace(contents, startIndex) {
  let index = startIndex;

  while (index >= 0) {
    if (!/\s/.test(contents[index])) {
      return index;
    }
    index -= 1;
  }

  return -1;
}

function isLikelyJsxTag(tagText) {
  return /^<\/?[A-Za-z][^<>]*>$/.test(tagText) || tagText === "<>" || tagText === "</>";
}

function readIdentifier(contents, startIndex) {
  let index = startIndex;
  while (index < contents.length && isIdentifierCharacter(contents[index])) {
    index += 1;
  }
  return contents.slice(startIndex, index);
}

function isIdentifierStart(character) {
  return /[$A-Z_a-z]/.test(character);
}

function isIdentifierCharacter(character) {
  return /[$0-9A-Z_a-z]/.test(character);
}
