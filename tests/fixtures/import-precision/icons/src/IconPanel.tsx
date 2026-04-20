import { FaBeer } from "react-icons";
import * as FaPack from "react-icons/fa";
import { FiAlertCircle } from "react-icons/fi";
import * as FiPack from "react-icons/fi";
import * as MaterialIcons from "@mui/icons-material";
import AddIcon from "@mui/icons-material/Add";

export function IconPanel() {
  return (
    <div>
      {FaBeer ? "beer" : "none"}
      {FaPack.FaRegBell ? "pack" : "none"}
      {FiAlertCircle ? "fi" : "none"}
      {FiPack.FiSettings ? "fi pack" : "none"}
      {MaterialIcons.Add ? "mui" : "none"}
      {AddIcon ? "single" : "none"}
    </div>
  );
}
