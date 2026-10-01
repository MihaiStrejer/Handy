import { chromium } from '@playwright/test';
import { fileURLToPath, pathToFileURL } from 'node:url';
import path from 'node:path';
const directory = path.dirname(fileURLToPath(import.meta.url));
const browser = await chromium.launch({ channel: 'msedge', headless: true });
const page = await browser.newPage();
const errors = [];
page.on('pageerror', error => errors.push(error.message));
for (const [name, width, height] of [['desktop', 1440, 1000], ['mobile', 390, 844]]) {
  await page.setViewportSize({ width, height });
  await page.goto(pathToFileURL(path.join(directory, 'index.html')).href);
  const result = await page.evaluate(() => ({
    overflow: document.documentElement.scrollWidth > innerWidth,
    exceptions: document.querySelectorAll('.strip').length,
    frames: document.querySelectorAll('.frame').length,
    brokenAnchors: [...document.querySelectorAll('a[href^="#"]')].filter(a => !document.getElementById(a.hash.slice(1))).map(a => a.hash),
    externalAssets: document.querySelectorAll('img[src^="http"],script[src],link[rel="stylesheet"]').length,
  }));
  if (result.overflow || result.exceptions !== 5 || result.frames !== 6 || result.brokenAnchors.length || result.externalAssets) throw new Error(JSON.stringify(result));
  await page.screenshot({ path: path.join(directory, `${name}.png`), fullPage: true });
  console.log(name, JSON.stringify(result));
}
await page.locator('a[href="#S3"]').first().click();
if (!await page.locator('#S3').evaluate(el => el.open)) throw new Error('Evidence link did not open source');
if (errors.length) throw new Error(errors.join('\n'));
await browser.close();
