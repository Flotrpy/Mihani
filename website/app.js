(function () {
  "use strict";

  var REPO = "Flotrpy/Mihani";
  var statusEl = document.getElementById("download-status");
  var windowsCard = document.getElementById("download-windows");
  var macosCard = document.getElementById("download-macos");

  function detectPlatform() {
    var platform = (navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || "";
    var ua = navigator.userAgent || "";
    if (/mac/i.test(platform) || /Macintosh|Mac OS X/i.test(ua)) return "macos";
    if (/win/i.test(platform) || /Windows/i.test(ua)) return "windows";
    return null;
  }

  function highlightDetected(platform) {
    if (platform === "windows") windowsCard.classList.add("detected");
    if (platform === "macos") macosCard.classList.add("detected");
  }

  function pickAsset(assets, matcher) {
    for (var i = 0; i < assets.length; i++) {
      if (matcher(assets[i].name)) return assets[i];
    }
    return null;
  }

  function applyAssetLink(card, asset) {
    if (!asset) return;
    var link = card.querySelector(".download-link");
    link.href = asset.browser_download_url;
    var meta = card.querySelector(".download-meta");
    var sizeMb = (asset.size / (1024 * 1024)).toFixed(1);
    meta.textContent = meta.textContent.split(" · ")[0] + " · " + sizeMb + " MB";
  }

  highlightDetected(detectPlatform());

  fetch("https://api.github.com/repos/" + REPO + "/releases/latest")
    .then(function (res) {
      if (!res.ok) throw new Error("HTTP " + res.status);
      return res.json();
    })
    .then(function (release) {
      var assets = release.assets || [];
      var windowsAsset = pickAsset(assets, function (name) {
        return /\.exe$/i.test(name);
      });
      var macosAsset = pickAsset(assets, function (name) {
        return /\.dmg$/i.test(name);
      });
      applyAssetLink(windowsCard, windowsAsset);
      applyAssetLink(macosCard, macosAsset);
      statusEl.textContent = "Latest release: " + release.tag_name;
    })
    .catch(function () {
      // No release published yet, or the API request failed (rate limit,
      // offline). The download buttons already fall back to the releases
      // page itself, so this degrades gracefully rather than breaking.
      statusEl.textContent = "Showing the releases page — could not load the latest release automatically.";
    });
})();
