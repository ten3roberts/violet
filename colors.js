const colors = { 'eerie_black': { DEFAULT: '#212121', 100: '#070707', 200: '#0d0d0d', 300: '#141414', 400: '#1b1b1b', 500: '#212121', 600: '#4e4e4e', 700: '#7a7a7a', 800: '#a6a6a6', 900: '#d3d3d3' }, 'platinum': { DEFAULT: '#e5e4e2', 100: '#302e2b', 200: '#5f5c56', 300: '#8e8a82', 400: '#b9b7b2', 500: '#e5e4e2', 600: '#eae9e7', 700: '#efeeed', 800: '#f4f4f3', 900: '#faf9f9' }, 'jade': { DEFAULT: '#53a66f', 100: '#112116', 200: '#21422c', 300: '#326443', 400: '#438559', 500: '#53a66f', 600: '#75b98c', 700: '#97cba8', 800: '#badcc5', 900: '#dceee2' }, 'dark_cyan': { DEFAULT: '#409999', 100: '#0d1f1f', 200: '#1a3e3e', 300: '#275d5d', 400: '#347c7c', 500: '#409999', 600: '#5bbaba', 700: '#84cccc', 800: '#addddd', 900: '#d6eeee' }, 'ultra_violet': { DEFAULT: '#534185', 100: '#110d1b', 200: '#211a35', 300: '#322750', 400: '#43356b', 500: '#534185', 600: '#6f58ad', 700: '#9382c1', 800: '#b7acd6', 900: '#dbd5ea' }, 'redwood': { DEFAULT: '#b35a4f', 100: '#241210', 200: '#49241f', 300: '#6d362f', 400: '#92483e', 500: '#b35a4f', 600: '#c37c73', 700: '#d29d96', 800: '#e1beb9', 900: '#f0dedc' }, 'lion': { DEFAULT: '#b38c49', 100: '#231c0e', 200: '#47381d', 300: '#6a532b', 400: '#8e6f3a', 500: '#b38c49', 600: '#c3a36b', 700: '#d2ba90', 800: '#e1d1b5', 900: '#f0e8da' } }

console.log(`use palette::Srgba;
use crate::srgba;
`)

for (var color_name in colors) {
  const tints = colors[color_name];
  for (var tint in tints) {
    const color = tints[tint];
    // console.log(`tint: ${ tint }, name: ${ name }, value: ${ color } `)
    console.log(
      `pub const ${color_name.toUpperCase()}_${tint.toUpperCase()}: Srgba = srgba!("${color}"); `,
    );
  }
}
