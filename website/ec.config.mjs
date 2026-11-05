// @ts-check
// Expressive Code (code blocks): themes, frames and colors. Read by astro.config.mjs and by
// the <Code> component on the landing page.
import { defineEcConfig } from "astro-expressive-code";

export default defineEcConfig({
  themes: ["github-dark-default", "github-light-default"],
  // <html data-theme="dark|light"> picks the theme; the media query covers no-JS readers.
  customizeTheme: (theme) => {
    theme.name = theme.type;
  },
  useDarkModeMediaQuery: true,
  themeCssSelector: (theme) => `[data-theme='${theme.type}']`,
  defaultProps: { wrap: false, frame: "code" },
  styleOverrides: {
    borderRadius: "0",
    borderColor: "var(--border)",
    borderWidth: "1px",
    codeBackground: "var(--code-bg)",
    codeFontFamily: "'JetBrains Mono Variable', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
    codeFontSize: "0.8125rem",
    codeLineHeight: "1.65",
    uiFontFamily: "'Inter Variable', Inter, system-ui, sans-serif",
    uiFontSize: "0.8125rem",
    focusBorder: "var(--primary)",
    scrollbarThumbColor: "var(--scrollbar)",
    scrollbarThumbHoverColor: "var(--muted)",
    frames: {
      shadowColor: "transparent",
      frameBoxShadowCssValue: "none",
      editorBackground: "var(--code-bg)",
      editorTabBarBackground: "var(--surface-2)",
      editorActiveTabBackground: "var(--code-bg)",
      editorActiveTabForeground: "var(--foreground)",
      editorActiveTabIndicatorTopColor: "var(--primary)",
      editorActiveTabIndicatorBottomColor: "transparent",
      editorTabBorderRadius: "0",
      editorTabBarBorderBottomColor: "var(--border)",
      terminalBackground: "var(--code-bg)",
      terminalTitlebarBackground: "var(--surface-2)",
      terminalTitlebarForeground: "var(--muted)",
      terminalTitlebarBorderBottomColor: "var(--border)",
      terminalTitlebarDotsForeground: "var(--muted)",
      terminalTitlebarDotsOpacity: "0.5",
      inlineButtonBackground: "var(--foreground)",
      inlineButtonForeground: "var(--foreground)",
      inlineButtonBorder: "var(--border)",
      tooltipSuccessBackground: "var(--primary)",
      tooltipSuccessForeground: "#ffffff",
    },
    textMarkers: {
      markBackground: "color-mix(in oklab, var(--primary) 16%, transparent)",
      markBorderColor: "var(--primary)",
    },
  },
});
