# Frame brand artwork

These files identify the Frame application. They are separate from the Nucleo interface icons in `lib/icons`.

| Asset | Source | Transformation |
| --- | --- | --- |
| `logo.svg` | Frame repository, `frame-app/assets/icons/frame.svg` | Original paths retained; the fixed fill was changed to `currentColor` for light and dark themes. |
| `apple-touch-icon.png` | Previous `frame-landing-page` static exports | Copied without changing the artwork. |
| `web-app-manifest-192x192.png` | Previous `frame-landing-page` static exports | Copied without changing the artwork. |
| `web-app-manifest-512x512.png` | Previous `frame-landing-page` static exports | Copied without changing the artwork. |
| `static/media/frame-app-icon.png` | User-provided `Icon-iOS-Default-1024x1024@1x.png` | Copied byte-for-byte at 1024 × 1024, preserving the 16-bit channels, Display P3 profile, and transparency. Used in the landing page's local-processing diagram. |

The original static exports were retained before replacing the old landing-page source. Their earlier binary export history is not recorded here.

The `/favicon.svg` endpoint renders the Frame mark from `logo.svg`. The web manifest and apple-touch icon metadata use the PNG assets above. Original standalone favicon exports are also retained in `static/`.

Source application: [66HEX/frame](https://github.com/66HEX/frame). Frame's application source is GPL-3.0 licensed; the MIT website-template notice does not relicense product artwork or grant trademark rights.
