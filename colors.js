const colors = {
  eerie_black: {
    '50': '#d9d9d9',
    '100': '#cccccc',
    '200': '#b5b5b5',
    '300': '#949494',
    '400': '#6b6b6b',
    '500': '#525252',
    '600': '#404040',
    '700': '#333333',
    '800': '#292929',
    '900': '#212121',
    '950': '#1b1b1b',
  },
  platinum: {
    '50': '#f6f6f6',
    '100': '#e5e4e2',
    '200': '#d6d4d2',
    '300': '#bcbab5',
    '400': '#a19c96',
    '500': '#8e8781',
    '600': '#817a75',
    '700': '#6c6662',
    '800': '#5a5552',
    '900': '#4a4744',
    '950': '#272423',
  },
  emerald: {
    '50': '#effaf2',
    '100': '#d9f2dd',
    '200': '#b5e5c1',
    '300': '#85d09b',
    '400': '#57b777',
    '500': '#309956',
    '600': '#207b44',
    '700': '#1a6238',
    '800': '#174e2e',
    '900': '#144028',
    '950': '#0a2416',
  },
  cyan: {
    '50': '#f3faf9',
    '100': '#d8efed',
    '200': '#b0dfdb',
    '300': '#80c8c4',
    '400': '#56aba9',
    '500': '#409999',
    '600': '#2e7173',
    '700': '#285b5d',
    '800': '#24494b',
    '900': '#213f40',
    '950': '#0f2224',
  },
  ultra_violet: {
    '50': '#f1f2fc',
    '100': '#e6e6f9',
    '200': '#d1d1f4',
    '300': '#b6b5ec',
    '400': '#9f97e2',
    '500': '#8d7dd7',
    '600': '#7e63c8',
    '700': '#6d53af',
    '800': '#534185',
    '900': '#4a3d72',
    '950': '#2c2442',
  },
  redwood: {
    '50': '#fbf6f5',
    '100': '#f8eae8',
    '200': '#f2d9d6',
    '300': '#e8beb9',
    '400': '#da978f',
    '500': '#c8756b',
    '600': '#b35a4f',
    '700': '#96493f',
    '800': '#7d3f37',
    '900': '#693933',
    '950': '#381b17',
  },
  lion: {
    '50': '#f8f6ee',
    '100': '#eee9d3',
    '200': '#ded2aa',
    '300': '#cbb479',
    '400': '#bb9954',
    '500': '#b38c49',
    '600': '#946c3a',
    '700': '#775131',
    '800': '#64442f',
    '900': '#573b2c',
    '950': '#321f16',
  },
};

console.log(`use palette::Srgba;
use crate::srgba;
`);

for (var color_name in colors) {
  const tints = colors[color_name];
  for (var tint in tints) {
    const color = tints[tint];
    // console.log(`tint: ${ tint }, name: ${ name }, value: ${ color } `)
    console.log(
      `pub const ${color_name.toUpperCase()}_${tint.toUpperCase()}: Srgba = srgba!("${color}"); `,
    );
  }

  const tint_names = Object.keys(tints).map((tint) => `    ${color_name.toUpperCase()}_${tint.toUpperCase()},\n`).join("");
  console.log(`pub const ${color_name.toUpperCase()}_TINTS: [Srgba; ${Object.keys(tints).length}] = [\n${tint_names}];`)
}
