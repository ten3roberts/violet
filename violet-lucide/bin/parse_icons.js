const fs = require('fs');
const path = require('path');

function toScreamingSnakeCase(str) {
    return str.replace(/-/g, '_').toUpperCase();
}

const filePath = path.join(__dirname, './lucide/info.json');
const rawData = fs.readFileSync(filePath, 'utf-8');
const jsonData = JSON.parse(rawData);

var results = Object.entries(jsonData).map(([key, value]) => {
    const screamingName = toScreamingSnakeCase(key);
    const utfValue = value.encodedCode.replace('\\', '')
    return `pub const ICON_${screamingName}: &str = "\\u{${utfValue}}";`;
});


// write `results` to file
const outputPath = path.join(__dirname, '../src/lucide_icons.rs');
fs.writeFileSync(outputPath, results.join('\n'), 'utf-8');
