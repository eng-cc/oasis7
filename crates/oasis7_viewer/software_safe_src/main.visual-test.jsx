// Direct offline fixtures own explicit local configuration. Live test pages
// require the launcher's trusted configuration exactly like release pages.
if (new URLSearchParams(location.search).get("connect") === "0" && !document.querySelector("#oasis7-viewer-runtime-config")) {
  const config = document.createElement("script");
  config.id = "oasis7-viewer-runtime-config";
  config.type = "application/json";
  config.textContent = JSON.stringify({ deploymentMode: "trusted_local_only", viewerWsEndpoint: "ws://127.0.0.1:9001/" });
  document.head.append(config);
}
Promise.all([import("./viewer_app.jsx"), import("./viewer_visual_test_assembly.js")]).then(([app, assembly]) => {
  app.configureViewerVisualTest(assembly.viewerVisualTestAdapter);
  app.mountViewerApp();
});
