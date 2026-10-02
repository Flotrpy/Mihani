# mihani.dev

The Mihani marketing/download site: a static page (no build step) covering
features, supported agents, the security model, and downloads for the
latest GitHub Release.

## Local preview

```bash
npx serve website
```

## Deploying

This directory deploys to Vercel as a static site — `vercel.json` sets
clean URLs, and there's no build command because the whole site is plain
HTML/CSS/JS. Point a Vercel project's root directory at `website/` with
no framework preset ("Other").

The download section calls `https://api.github.com/repos/Flotrpy/Mihani/releases/latest`
client-side to link directly to the current Windows/macOS installer
assets, falling back to the releases page itself if that request fails
(rate-limited, offline, or no release published yet).
