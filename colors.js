colors = require("colors.json")

console.log(`use palette::Srgba;
use crate::srgba;
`);

function kebabToSnakeCase(kebabCaseString) {
  return kebabCaseString.replace(/-/g, '_').toUpperCase();
}

for (var color_name in colors) {
  const tints = colors[color_name];
  for (var tint in tints) {
    const color = tints[tint];
    let color_name = kebabToSnakeCase(color_name);
    // console.log(`tint: ${ tint }, name: ${ name }, value: ${ color } `)
    console.log(
      `pub const ${color_name}_${tint.toUpperCase()}: Srgba = srgba!("${color}"); `,
    );
  }

  const tint_names = Object.keys(tints).map((tint) => `    ${color_name}_${tint.toUpperCase()},\n`).join("");
  console.log(`pub const ${color_name}_TINTS: [Srgba; ${Object.keys(tints).length}] = [\n${tint_names}];`)
}
