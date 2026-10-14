import { createRoot } from "react-dom/client";
import Activity from "./Activity";

const root = document.getElementById("root");
if (!root) throw new Error("Missing #root element");
createRoot(root).render(<Activity />);
