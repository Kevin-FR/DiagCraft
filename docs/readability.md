# Interface readability

DiagCraft 0.11.3 separates informational text from muted decoration. The drawing
format remains v7 and the agent API remains v1. Document text, fills, strokes,
previews and exported colours remain under the author's control.

## Roles and criteria

The engineering targets for enabled interface text are **4.5:1** against its
actual background, and **3:1** for meaningful non-text state indicators. We use
the sRGB relative-luminance calculation and compare unrounded ratios. These
reference values come from W3C's explanations of [text contrast](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html)
and [non-text contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html).
They are test criteria for this work, not a claim of complete WCAG conformance
or a replacement for testing a native application with assistive technology.

| Role | Current uses and logical font sizes | Backgrounds / states |
| --- | --- | --- |
| Informational text `#526271` | Footer 11 pt; property captions 10 pt; inspector help 12 pt; navigation hint 11 pt; secondary search/library/dialog text uses the small text style | White panels, pale ribbon and tabs, canvas `#f1f4f7`, hovered and selected search rows |
| Ruler text `#526271` | Horizontal values 10 pt, vertical values 8 pt, unit 9 pt | Pale ruler strip; tick marks retain their decorative colour |
| Placeholders `#526271` | Empty search and text fields; input font size | Field fills; explicit weak-text colour avoids applying the default translucent weak-text treatment |
| Actions `#0053b4` | Enabled blue text, links and active ribbon labels; 12 pt connection hints | Pale controls or opaque white hint; normal, hover and selected text where used |
| Filled actions `#0053b4` | White label on a primary action button | Opaque blue fill |
| Warnings `#7e4e00`, errors `#a62525` | Agent, routing, recovery, image and Mermaid information | Light panel and modal fills |
| Empty-page invitation | 23 pt title and 13 pt instruction | User-selected opaque RGB page fill; keep the informational colour if it reaches 4.5:1, otherwise choose black or white |
| Focus outline | Existing `#0053b4` outline | Control and canvas backgrounds; target at least 3:1 |
| Unsaved-document indicator `#ca4242` | Dot identifying modified tabs | Active and inactive tab fills; target at least 3:1 |

Informational text measures approximately 6.28:1 on white, 5.69:1 on the canvas
and 5.34:1 on the selected-row fill. The unsaved indicator has a minimum of
approximately 3.84:1 on the two tab fills. These rounded figures are descriptive;
tests compare the underlying values.

Muted decoration, inactive controls and logo colours are not converted globally.
Disabled actions remain visually distinct and inert. This distinction avoids
changing user-authored diagram colours or making unavailable actions look enabled.
The small ruler typography remains small; its practical readability still needs
human assessment at actual display scaling.

## Hints on the canvas

The navigation hint has an opaque canvas-coloured capsule and wraps within the
available width. It stays legible when a dark page is panned underneath it.
Connection hints use an opaque white capsule, wrap to the canvas width and clamp
their position inside the canvas. Their clip is the canvas, independently of the
page clip used to paint a connector, so a free-endpoint hint remains visible
outside the sheet. Valid, invalid and free-endpoint hints use enabled action or
error text colours; port and connector geometry retain their colours.

These aids are interface overlays. They do not alter the document or appear in
its SVG, PDF or PNG export.

## Automated checks and remaining work

Run the seven painting regressions with:

```sh
cargo test --locked -p diagcraft contrast_tests
```

The checks exercise a 1200 × 680 logical-point content area in all seven interface
languages. They inspect actual glyph mesh colours, text colour overrides, opacity
and the rectangular fills painted behind the text, rather than only comparing
palette constants. They cover status/help/property captions, real placeholders,
search details in normal/hovered/selected/focused states, disabled copy behaviour,
focus outlines, modified-tab indicators and valid/invalid/free connection hints.
They also cover a panned dark page and six page backgrounds: white, dark,
mid-grey, orange, blue and green. Hint clipping and document/history preservation
are checked alongside colour ratios.

Search-window animations are disabled in these tests to measure **settled
states**. The measurements use glyph colours before raster anti-aliasing and
sample the existing flat backgrounds; they are not an exhaustive pixel audit of
all screens, animation transitions, display profiles or future textured fills.

Still required for [issue #9](https://github.com/Kevin-FR/DiagCraft/issues/9):

- Review at real small-screen display scaling, including the ruler typography,
  hover/focus transitions and the create → connect → save → reopen → export workflow.
- Check announcements and operation with native screen readers on supported desktops.
- Obtain human feedback, including translation review, and record any remaining barriers.

See the [keyboard guide](keyboard-navigation.md) and [quality scenarios](quality.md).
