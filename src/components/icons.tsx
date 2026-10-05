/**
 * Inline SVG icons, the same vocabulary as Diskette's
 * (src/components/icons.tsx there). currentColor throughout so every icon
 * themes for free. These are stand-ins until the theme pack's ANSI icons,
 * drawn in Stylus, replace them (Diskette's docs/ansiapps-theme.md).
 *
 * AppMarkIcon is the header's mark, the same drawing as the app icon
 * (src-tauri/icons/source-icon.svg is the master, run through `tauri icon`
 * for every platform size), cropped to its handset cups. It's the one glyph with
 * fixed colors: a brand mark, not a themeable UI icon.
 */
import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;

export function AppMarkIcon(props: IconProps) {
  return (
    <svg viewBox="76 176 360 160" aria-hidden="true" {...props}>
      <g fill="#ffff55">
        <rect x="96" y="176" width="120" height="40" />
        <rect x="76" y="216" width="160" height="120" />
        <rect x="296" y="176" width="120" height="40" />
        <rect x="276" y="216" width="160" height="120" />
      </g>
      <g fill="#0000aa">
        <rect x="116" y="256" width="80" height="40" />
        <rect x="316" y="256" width="80" height="40" />
      </g>
      <rect x="236" y="296" width="40" height="14" fill="#ffffff" />
    </svg>
  );
}

export function GearIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true" {...props}>
      <circle cx="12" cy="12" r="3" fill="none" stroke="currentColor" strokeWidth={2} />
      <path
        fill="none"
        stroke="currentColor"
        strokeWidth={2}
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
      />
    </svg>
  );
}

export function ChecklistIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true" {...props}>
      <rect x="4" y="3" width="16" height="18" rx="2" fill="none" stroke="currentColor" strokeWidth={2} />
      <path fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" d="M8 8.5l1.5 1.5L12 7" />
      <path stroke="currentColor" strokeWidth={2} strokeLinecap="round" d="M14.5 9h3" />
      <path fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" d="M8 14.5l1.5 1.5L12 13" />
      <path stroke="currentColor" strokeWidth={2} strokeLinecap="round" d="M14.5 15h3" />
    </svg>
  );
}

export function InfoIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true" {...props}>
      <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" strokeWidth={2} />
      <path stroke="currentColor" strokeWidth={2} strokeLinecap="round" d="M12 11v5.5" />
      <circle cx="12" cy="8" r="1" fill="currentColor" />
    </svg>
  );
}
