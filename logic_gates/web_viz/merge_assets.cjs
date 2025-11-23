const fs = require('fs');
const path = require('path');

const distDir = path.join(__dirname, 'dist', 'assets');
const outDir = path.join(__dirname, 'dist');

try {
    const jsPath = path.join(distDir, 'index.js');
    const cssPath = path.join(distDir, 'style.css');
    
    if (!fs.existsSync(jsPath) || !fs.existsSync(cssPath)) {
        console.error('Build files not found in dist/assets/. Please run `npm run build` first.');
        process.exit(1);
    }

    const js = fs.readFileSync(jsPath, 'utf8');
    const css = fs.readFileSync(cssPath, 'utf8');

    // Simple CSS injection script
    // Escape backticks and backslashes for template literal
    const escapedCss = css
        .replace(/\\/g, '\\\\')
        .replace(/`/g, '\`')
        .replace(/\$/g, '\\$');

    const embedScript = `
(function() {
  var style = document.createElement('style');
  style.textContent = 
${escapedCss}
;
  document.head.appendChild(style);
})();
${js}
`;

    fs.writeFileSync(path.join(outDir, 'logic_gates_embed.js'), embedScript);
    console.log('Successfully created logic_gates_embed.js');

} catch (e) {
    console.error(e);
    process.exit(1);
}

