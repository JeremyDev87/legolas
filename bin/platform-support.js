export function resolveCurrentHostSupport() {
  return resolveHostSupport(process.platform, process.arch, {
    linuxGlibc: hasLinuxGlibcRuntime()
  });
}

export function resolveHostSupport(platform, arch, options = {}) {
  const hostKey = `${platform}/${arch}`;

  switch (hostKey) {
    case "darwin/arm64":
      return supportedHost(hostKey, "aarch64-apple-darwin");
    case "darwin/x64":
      return supportedHost(hostKey, "x86_64-apple-darwin");
    case "linux/x64":
      if (options.linuxGlibc === false) {
        return unsupportedHost(hostKey);
      }
      return supportedHost(hostKey, "x86_64-unknown-linux-gnu");
    case "win32/x64":
      return supportedHost(hostKey, "x86_64-pc-windows-msvc");
    default:
      return unsupportedHost(hostKey);
  }
}

export function missingBinaryMessage(hostKey) {
  return `legolas: no packaged Rust binary for ${hostKey}`;
}

function supportedHost(hostKey, targetTriple) {
  return {
    supported: true,
    hostKey,
    targetTriple,
    binaryName: targetTriple.includes("windows") ? "legolas.exe" : "legolas"
  };
}

function unsupportedHost(hostKey) {
  return {
    supported: false,
    hostKey,
    targetTriple: null,
    binaryName: null
  };
}

function hasLinuxGlibcRuntime() {
  if (process.platform !== "linux") {
    return true;
  }

  const runtimeReport = process.report?.getReport?.();
  const runtimeHeader = runtimeReport?.header;

  return Boolean(runtimeHeader?.glibcVersionRuntime || runtimeHeader?.glibcVersionCompiler);
}
