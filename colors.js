colors = require("./colors.json")

console.log(`use palette::Srgba;
use crate::srgba;
`);

function kebabToSnakeCase(kebabCaseString) {
  return kebabCaseString.replace(/-/g, '_').toUpperCase();
}

for (var color_name in colors) {
  let uppercase_name = kebabToSnakeCase(color_name);
  const tints = colors[color_name];
  for (var tint in tints) {
    const color = tints[tint];

    // console.log(`tint: ${ tint }, uppercase_name: ${ uppercase_name }, value: ${ color } `)
    console.log(
      `pub const ${uppercase_name}_${tint.toUpperCase()}: Srgba = srgba!("${color}"); `,
    );
  }

}

for (var color_name in colors) {
  let uppercase_name = kebabToSnakeCase(color_name);
  const tints = colors[color_name];
  const tint_names = Object.keys(tints).map((tint) => `    ${uppercase_name}_${tint.toUpperCase()},\n`).join("");
  console.log(`pub const ${uppercase_name}_TINTS: [Srgba; ${Object.keys(tints).length}] = [\n${tint_names}];`)
}
