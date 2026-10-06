const { test } = require('node:test');
const assert = require('node:assert/strict');
const postcss = require('postcss');
const tailwind = require('tailwindcss');
const resolveConfig = require('tailwindcss/resolveConfig');
const config = require('../tailwind.config');
const { theme } = resolveConfig(config);

function luminance(hex) {
  let digits = hex.replace('#', '');
  if (digits.length === 3) digits = [...digits].map(x => x + x).join('');
  const rgb = digits.match(/../g).map(x => parseInt(x, 16) / 255)
    .map(v => v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4);
  return rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
}
function contrast(a, b) {
  const values = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (values[0] + 0.05) / (values[1] + 0.05);
}

test('body text and status labels stay readable on dark surfaces', () => {
  for (const surface of [theme.backgroundColor.white, theme.backgroundColor.gray[50]]) {
    assert.ok(luminance(surface) < 0.05);
    for (const shade of [500, 600, 700, 800, 900]) {
      assert.ok(contrast(theme.textColor.gray[shade], surface) >= 4.5);
    }
  }
  for (const name of ['red', 'green', 'yellow', 'blue', 'amber', 'orange']) {
    for (const shade of [600, 700, 800, 900]) {
      assert.ok(contrast(theme.textColor[name][shade], theme.backgroundColor[name][50]) >= 4.5);
    }
  }
  assert.equal(theme.textColor.white, '#fff');
  assert.ok(contrast(theme.textColor.white, theme.backgroundColor.blue[600]) >= 4.5);
});

test('Tailwind applies the palette to hover, disabled, opacity, and gradients', async () => {
  const result = await postcss([tailwind({ ...config, content: [{ raw:
    '<div class="bg-white text-white bg-white/50 hover:bg-gray-100 disabled:text-gray-500 from-blue-50 to-purple-50 border-gray-200"></div>',
  }] })]).process('@tailwind utilities;', { from: undefined });
  const rules = new Map();
  result.root.walkRules(rule => {
    const props = {};
    rule.walkDecls(decl => { props[decl.prop] = decl.value; });
    rules.set(rule.selector, props);
  });
  assert.match(rules.get('.bg-white')['background-color'], /31 41 55/);
  assert.match(rules.get('.text-white').color, /255 255 255/);
  assert.match(rules.get('.bg-white\\/50')['background-color'], /31 41 55 \/ 0.5/);
  assert.match(rules.get('.hover\\:bg-gray-100:hover')['background-color'], /38 50 68/);
  assert.match(rules.get('.disabled\\:text-gray-500:disabled').color, /156 163 175/);
  assert.match(rules.get('.from-blue-50')['--tw-gradient-from'], /#172554/);
  assert.match(rules.get('.border-gray-200')['border-color'], /51 65 85/);
});
