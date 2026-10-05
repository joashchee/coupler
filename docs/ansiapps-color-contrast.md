# ANSIapps palette: contrast of every color pair

**For every ansiapps app's ANSIapps theme** (`docs/ansiapps-theme.md`).
Worked out 2026-09-29, when a Handler inside an archive turned out
unreadable in Diskette (light magenta on brown, 2.0:1). Pick text and
background pairs from the ranked list below. Don't eyeball them.

## The rule

- **Text** (anything a person reads, including a single status
  character) needs **4.5:1** (WCAG 2.2 AA, 1.4.3). The VGA font is 16px
  with no bold, so the "large text" 3:1 allowance never applies.
- **7:1** is AAA (1.4.6). Prefer an AAA pair for body text and long
  lists. Use an AA pair for short labels, badges and accents.
- **Icons, borders and focus rings** need **3:1** against what's next to
  them (1.4.11). The "3:1 only" list at the end is for those, never for
  words.
- **Disabled controls** are exempt (dark gray on light gray, 3.2:1, is
  the text-mode convention for them). Say they're disabled some other
  way too, such as the `disabled` attribute.
- **Color is never the only cue** (1.4.1). A status dot also has a
  tooltip and an `aria-label`. A Handler row is also named in Details.

Ratios use the WCAG relative-luminance formula on the 16 CGA/VGA colors.
The script is at the end, so the numbers can be checked again.

## Ranked list: the 32 pairs that pass

16 colors make 240 text/background combinations. 64 of them pass AA,
and contrast is the same either way round, so that's 32 pairs, each
listed once. The other 176 combinations fail and are left out.

| Rank | Pair (either way round) | Ratio | Level | Notes |
|---|---|---|---|---|
| 1 | Black / White | 21.00:1 | AAA | |
| 2 | Black / Yellow | 19.69:1 | AAA | Selected Handler row (yellow on black) |
| 3 | Black / Light cyan | 17.13:1 | AAA | Primary button, ANSIapps location dot while checking |
| 4 | Black / Light green | 15.82:1 | AAA | Location dot "found" (on its black cell) |
| 5 | Blue / White | 13.29:1 | AAA | Main text on the blue desktop |
| 6 | Blue / Yellow | 12.46:1 | AAA | Code, Catalog notice |
| 7 | Blue / Light cyan | 10.84:1 | AAA | Frames, accents, breadcrumbs |
| 8 | Blue / Light green | 10.01:1 | AAA | |
| 9 | Black / Light gray | 9.04:1 | AAA | Buttons, menus and dialogs, search field |
| 10 | Black / Light magenta | 8.00:1 | AAA | |
| 11 | Red / White | 7.75:1 | AAA | Error bar, text danger buttons, row errors |
| 12 | Dark gray / White | 7.46:1 | AAA | |
| 13 | Black / Cyan | 7.33:1 | AAA | Selection bar (black on cyan) |
| 14 | Red / Yellow | 7.27:1 | AAA | |
| 15 | Dark gray / Yellow | 6.99:1 | AA | |
| 16 | Black / Green | 6.75:1 | AA | Status message, menu highlight |
| 17 | Black / Light red | 6.68:1 | AA | Location dot "missing" (on its black cell) |
| 18 | Magenta / White | 6.38:1 | AA | |
| 19 | Red / Light cyan | 6.32:1 | AA | |
| 20 | Dark gray / Light cyan | 6.08:1 | AA | |
| 21 | Magenta / Yellow | 5.98:1 | AA | |
| 22 | Red / Light green | 5.84:1 | AA | |
| 23 | Blue / Light gray | 5.72:1 | AA | Muted text on blue, muted text in menus |
| 24 | Dark gray / Light green | 5.62:1 | AA | |
| 25 | Brown / White | 5.24:1 | AA | |
| 26 | Magenta / Light cyan | 5.21:1 | AA | |
| 27 | Light blue / White | 5.09:1 | AA | |
| 28 | Blue / Light magenta | 5.06:1 | AA | Items inside an archive or `.app` |
| 29 | Brown / Yellow | 4.91:1 | AA | Handlers (Diskette's reserved gold) |
| 30 | Magenta / Light green | 4.81:1 | AA | |
| 31 | Light blue / Yellow | 4.77:1 | AA | |
| 32 | Blue / Cyan | 4.64:1 | AA | Barely passes. Avoid for long text |

What the ranking means in practice:

- **Blue desktop** (the main surface): white, yellow, light cyan, light
  green, light gray, light magenta, and nothing else. Light red on blue
  is **4.23:1 and fails**, so never use it for a word on blue. An error
  on blue is a white-on-red bar instead.
- **Light-gray windows** (menus, dialogs, buttons): only black and blue
  text. Red on light gray is **3.34:1 and fails**, so a danger button
  with a word on it is a red bar with white text.
- **Cyan selection bar**: only black text (blue is 4.64:1, too close).
  Anything colored in a selected row needs its own black cell or a
  black row, as Diskette's selected Handler row does.
- **Brown** (the Handler tint): only yellow and white. Light gray,
  light magenta and black all fail on brown.
- **Green** (menu highlight, status message): only black. Blue on green
  is 4.27:1, so gray-blue "muted" text turns black on a highlighted
  menu item.
- **Black cells** take every bright color. That's why a status character
  (a location dot, for example) sits on its own black cell: it stays
  readable on every row color, selected or not.

## Pairs for icons and borders only (3:1 to 4.5:1)

Never for text. Each pair works either way round.

| Pair | Ratio |
|---|---|
| Brown / Light cyan | 4.27:1 |
| Blue / Green | 4.27:1 |
| Blue / Light red | 4.23:1 |
| Light blue / Light cyan | 4.15:1 |
| Black / Light blue | 4.13:1 |
| Black / Brown | 4.01:1 |
| Brown / Light green | 3.95:1 |
| Light blue / Light green | 3.83:1 |
| Red / Light gray | 3.34:1 |
| Black / Magenta | 3.29:1 |
| Dark gray / Light gray | 3.21:1 (disabled controls) |
| Light red / White | 3.14:1 |
| Green / White | 3.11:1 |

## Full matrix

Text color down the side, background across the top. **Bold** is AAA,
plain is AA, ✗ fails text.

| fg \ bg | Blk | Blu | Grn | Cyn | Red | Mag | Brn | LGy | DGy | LBl | LGn | LCy | LRd | LMg | Yel | Wht |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Black | — | ✗ | 6.7 | **7.3** | ✗ | ✗ | ✗ | **9.0** | ✗ | ✗ | **15.8** | **17.1** | 6.7 | **8.0** | **19.7** | **21.0** |
| Blue | ✗ | — | ✗ | 4.6 | ✗ | ✗ | ✗ | 5.7 | ✗ | ✗ | **10.0** | **10.8** | ✗ | 5.1 | **12.5** | **13.3** |
| Green | 6.7 | ✗ | — | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Cyan | **7.3** | 4.6 | ✗ | — | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Red | ✗ | ✗ | ✗ | ✗ | — | ✗ | ✗ | ✗ | ✗ | ✗ | 5.8 | 6.3 | ✗ | ✗ | **7.3** | **7.8** |
| Magenta | ✗ | ✗ | ✗ | ✗ | ✗ | — | ✗ | ✗ | ✗ | ✗ | 4.8 | 5.2 | ✗ | ✗ | 6.0 | 6.4 |
| Brown | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | — | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | 4.9 | 5.2 |
| Light gray | **9.0** | 5.7 | ✗ | ✗ | ✗ | ✗ | ✗ | — | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ |
| Dark gray | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | — | ✗ | 5.6 | 6.1 | ✗ | ✗ | 7.0 | **7.5** |
| Light blue | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | — | ✗ | ✗ | ✗ | ✗ | 4.8 | 5.1 |
| Light green | **15.8** | **10.0** | ✗ | ✗ | 5.8 | 4.8 | ✗ | ✗ | 5.6 | ✗ | — | ✗ | ✗ | ✗ | ✗ | ✗ |
| Light cyan | **17.1** | **10.8** | ✗ | ✗ | 6.3 | 5.2 | ✗ | ✗ | 6.1 | ✗ | ✗ | — | ✗ | ✗ | ✗ | ✗ |
| Light red | 6.7 | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | — | ✗ | ✗ | ✗ |
| Light magenta | **8.0** | 5.1 | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | ✗ | — | ✗ | ✗ |
| Yellow | **19.7** | **12.5** | ✗ | ✗ | **7.3** | 6.0 | 4.9 | ✗ | 7.0 | 4.8 | ✗ | ✗ | ✗ | ✗ | — | ✗ |
| White | **21.0** | **13.3** | ✗ | ✗ | **7.8** | 6.4 | 5.2 | ✗ | **7.5** | 5.1 | ✗ | ✗ | ✗ | ✗ | ✗ | — |

## Diskette's audit (2026-09-29)

Every pair Diskette's ANSIapps theme used, checked against the list:

| Where | Was | Now |
|---|---|---|
| Handler row or tile inside an archive | Light magenta on brown, 2.0:1 | Yellow on brown, 4.9:1 |
| Selected Handler row inside an archive | Black on black (invisible) | Yellow on black, 19.7:1 |
| Selected Handler tile | Yellow on cyan, 2.7:1 | Yellow on black, 19.7:1 |
| Size/date on a Handler row | Light gray on brown, fails | Yellow on brown, 4.9:1 |
| Text danger buttons (Delete) | Red on light gray, 3.3:1 | White on red, 7.8:1 |
| Gray-blue text on a highlighted menu item | Blue on green, 4.3:1 | Black on green, 6.8:1 |
| Everything else (selection bar, panels, dialogs, error and status bars, archive tint, progress) | Passes | Unchanged |

Dev-only (App Testing's bug notes in light red on blue, 4.2:1) and
disabled controls were left alone.

## Checking a new pair

Before adding a colored element to an ANSIapps theme, look up its text
and background here. If it has to sit on several backgrounds (a row
that can be selected, busy, or tinted), give it its own black cell or
check every background it can land on.

```python
pal = {"Black": "000000", "Blue": "0000AA", "Green": "00AA00", "Cyan": "00AAAA",
       "Red": "AA0000", "Magenta": "AA00AA", "Brown": "AA5500", "Light gray": "AAAAAA",
       "Dark gray": "555555", "Light blue": "5555FF", "Light green": "55FF55",
       "Light cyan": "55FFFF", "Light red": "FF5555", "Light magenta": "FF55FF",
       "Yellow": "FFFF55", "White": "FFFFFF"}

def lum(h):
    c = [int(h[i:i + 2], 16) / 255 for i in (0, 2, 4)]
    c = [x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4 for x in c]
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]

def ratio(a, b):
    hi, lo = sorted((lum(pal[a]), lum(pal[b])), reverse=True)
    return (hi + 0.05) / (lo + 0.05)

print(f"{ratio('Yellow', 'Brown'):.2f}:1")  # 4.91:1
```
