import type { GlobalThemeOverrides } from "naive-ui";

const font =
  '"Inter", "Segoe UI", "Microsoft YaHei UI", "PingFang SC", system-ui, -apple-system, sans-serif';
const mono = '"Cascadia Code", "JetBrains Mono", "Consolas", ui-monospace, monospace';

const shared = {
  fontFamily: font,
  fontFamilyMono: mono,
  fontSize: "14px",
  fontSizeSmall: "12.5px",
  heightSmall: "30px",
  heightMedium: "38px",
  heightLarge: "42px",
  borderRadius: "10px",
  borderRadiusSmall: "8px",
  primaryColor: "#fda5d3",
  primaryColorHover: "#ffc0e2",
  primaryColorPressed: "#ef8fc2",
  primaryColorSuppl: "#f27cb6",
} as const;

/**
 * naive-ui 主题覆盖：亮色与暗色各自一套，配色以 #fda5d3 为主色派生。
 * 用一次断言规避全局主题类型对嵌套键的严格校验（naive-ui 会静默忽略未知键）。
 */
export function getThemeOverrides(isDark: boolean): GlobalThemeOverrides {
  const common = isDark
    ? {
        ...shared,
        bodyColor: "#130d12",
        cardColor: "#1e1520",
        modalColor: "#221826",
        popoverColor: "#221826",
        tableColor: "#1e1520",
        textColorBase: "#f7edf3",
        textColor1: "#f7edf3",
        textColor2: "#cdb8c6",
        textColor3: "#9a8192",
        borderColor: "rgba(253, 165, 211, 0.16)",
        dividerColor: "rgba(253, 165, 211, 0.10)",
        inputColor: "rgba(255, 255, 255, 0.045)",
        inputColorDisabled: "rgba(255, 255, 255, 0.03)",
        actionColor: "rgba(253, 165, 211, 0.08)",
        hoverColor: "rgba(253, 165, 211, 0.12)",
        successColor: "#6ee7b7",
        warningColor: "#fcd34d",
        errorColor: "#fb7185",
        infoColor: "#fda5d3",
      }
    : {
        ...shared,
        bodyColor: "#fdf7fb",
        cardColor: "#ffffff",
        modalColor: "#ffffff",
        popoverColor: "#ffffff",
        tableColor: "#ffffff",
        textColorBase: "#2b1822",
        textColor1: "#2b1822",
        textColor2: "#5b4551",
        textColor3: "#7a5f70",
        borderColor: "rgba(150, 90, 120, 0.18)",
        dividerColor: "rgba(150, 90, 120, 0.12)",
        inputColor: "#ffffff",
        inputColorDisabled: "#f7eef3",
        actionColor: "rgba(150, 90, 120, 0.05)",
        hoverColor: "rgba(253, 165, 211, 0.18)",
        successColor: "#0f9f74",
        warningColor: "#b45309",
        errorColor: "#e11d48",
        infoColor: "#c9377c",
      };

  const inputColor = isDark ? "rgba(255, 255, 255, 0.045)" : "#ffffff";
  const inputBorder = isDark
    ? "1px solid rgba(253, 165, 211, 0.16)"
    : "1px solid rgba(150, 90, 120, 0.18)";
  const inputBorderHover = isDark
    ? "1px solid rgba(253, 165, 211, 0.32)"
    : "1px solid rgba(226, 86, 143, 0.34)";
  const inputBorderFocus = "1px solid rgba(242, 124, 182, 0.85)";
  const focusRing = "0 0 0 3px rgba(253, 165, 211, 0.22)";

  const overrides = {
    common,
    Button: {
      borderRadiusSmall: "8px",
      borderRadiusMedium: "10px",
      borderRadiusLarge: "11px",
      fontWeight: "550",
      paddingMedium: "0 16px",
      colorPrimary: "#fda5d3",
      colorHoverPrimary: "#ffc0e2",
      colorPressedPrimary: "#ef8fc2",
      textColorPrimary: "#3a0c22",
      textColorHoverPrimary: "#3a0c22",
      textColorPressedPrimary: "#3a0c22",
      colorSecondary: isDark ? "rgba(253, 165, 211, 0.08)" : "rgba(150, 90, 120, 0.06)",
      colorSecondaryHover: isDark ? "rgba(253, 165, 211, 0.14)" : "rgba(150, 90, 120, 0.1)",
      colorSecondaryPressed: isDark ? "rgba(253, 165, 211, 0.18)" : "rgba(150, 90, 120, 0.14)",
      borderSecondary: `1px solid ${
        isDark ? "rgba(253, 165, 211, 0.18)" : "rgba(150, 90, 120, 0.2)"
      }`,
      textColorSecondary: isDark ? "#e8d6e1" : "#4a3340",
    },
    Input: {
      borderRadius: "10px",
      color: inputColor,
      colorFocus: inputColor,
      border: inputBorder,
      borderHover: inputBorderHover,
      borderFocus: inputBorderFocus,
      boxShadowFocus: focusRing,
      textColor: isDark ? "#f7edf3" : "#2b1822",
      placeholderColor: isDark ? "#8a7182" : "#8f7885",
      caretColor: "#f27cb6",
    },
    InternalSelection: {
      borderRadius: "10px",
      color: inputColor,
      colorActive: inputColor,
      border: inputBorder,
      borderHover: inputBorderHover,
      borderActive: inputBorderFocus,
      boxShadowActive: focusRing,
      textColor: isDark ? "#f7edf3" : "#2b1822",
      placeholderColor: isDark ? "#8a7182" : "#8f7885",
    },
    Select: {
      menuBoxShadow: isDark
        ? "0 18px 40px -18px rgba(0, 0, 0, 0.85)"
        : "0 18px 40px -18px rgba(120, 60, 90, 0.28)",
    },
    // 选项配色实际由内部组件 InternalSelectMenu 提供（Select / Dropdown 共用）
    InternalSelectMenu: {
      optionHeightMedium: "34px",
      optionTextColor: isDark ? "#cdb8c6" : "#4a3340",
      optionTextColorActive: isDark ? "#ffc4e3" : "#8e2350",
      optionTextColorPressed: isDark ? "#ffc4e3" : "#7d1c45",
      optionCheckColor: isDark ? "#fda5d3" : "#c9377c",
      optionColorPending: isDark ? "rgba(253, 165, 211, 0.14)" : "rgba(253, 165, 211, 0.28)",
      optionColorActive: isDark ? "rgba(253, 165, 211, 0.1)" : "rgba(253, 165, 211, 0.2)",
      optionColorActivePending: isDark ? "rgba(253, 165, 211, 0.18)" : "rgba(253, 165, 211, 0.34)",
    },
    Switch: {
      railColorActive: "#f27cb6",
      railColor: isDark ? "rgba(253, 165, 211, 0.2)" : "rgba(150, 90, 120, 0.2)",
    },
    Dialog: {
      borderRadius: "16px",
      color: isDark ? "#221826" : "#ffffff",
      titleFontSize: "16px",
      padding: "22px",
    },
    Message: {
      borderRadius: "11px",
    },
    Notification: {
      borderRadius: "13px",
      color: isDark ? "#221826" : "#ffffff",
    },
  } as unknown as GlobalThemeOverrides;

  return overrides;
}
