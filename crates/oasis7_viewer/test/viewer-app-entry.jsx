import { configureViewerVisualTest, mountViewerApp } from "../software_safe_src/viewer_app.jsx";
import { viewerVisualTestAdapter } from "../software_safe_src/viewer_visual_test_assembly.js";
configureViewerVisualTest(viewerVisualTestAdapter);
export * from "../software_safe_src/viewer_app.jsx";
const root = document.getElementById("app");
if (root) mountViewerApp(root);
