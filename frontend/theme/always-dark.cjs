const colors = require('tailwindcss/colors');

// This distribution is always dark. Override each utility's purpose separately:
// bg-white is a surface, while text-white must stay white on colored buttons.
const neutrals = ['gray', 'slate', 'zinc', 'neutral', 'stone'];
const accents = ['blue', 'red', 'green', 'yellow', 'amber', 'orange', 'purple',
  'indigo', 'cyan', 'teal', 'emerald', 'pink', 'rose', 'violet', 'sky', 'lime', 'fuchsia'];

const backgroundColor = { white: colors.gray[800] };
const textColor = { black: colors.gray[100] };
const borderColor = {};
const gradientColorStops = {};

for (const name of neutrals) {
  backgroundColor[name] = {
    50: colors.gray[900], 100: '#263244', 200: colors.slate[700],
    300: colors.slate[600], 400: colors.slate[500],
  };
  textColor[name] = {
    500: colors.gray[400], 600: colors.gray[300], 700: colors.gray[200],
    800: colors.gray[100], 900: colors.gray[50], 950: colors.gray[50],
  };
  borderColor[name] = {
    50: colors.slate[700], 100: colors.slate[700], 200: colors.slate[700],
    300: colors.slate[600], 400: colors.slate[500],
  };
}

for (const name of accents) {
  const palette = colors[name];
  backgroundColor[name] = { 50: palette[950], 100: palette[900], 200: palette[800] };
  textColor[name] = { 500: palette[400], 600: palette[300], 700: palette[300],
    800: palette[200], 900: palette[200], 950: palette[100] };
  borderColor[name] = { 100: palette[900], 200: palette[800], 300: palette[700] };
  gradientColorStops[name] = { 50: palette[950], 100: palette[900], 200: palette[800] };
}

module.exports = { backgroundColor, textColor, borderColor, gradientColorStops };
