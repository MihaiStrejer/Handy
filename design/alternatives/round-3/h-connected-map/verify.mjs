import { chromium, expect } from '@playwright/test';
import { existsSync } from 'node:fs';
import { writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';

const folder = resolve('design/alternatives/round-3/h-connected-map');
const browser = await chromium.launch({ channel: 'msedge', headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, javaScriptEnabled: false });
  const requests = [];
  page.on('request', request => { if (/^https?:/.test(request.url())) requests.push(request.url()); });
  await page.goto(pathToFileURL(`${folder}/index.html`).href);
  const result = await page.evaluate(() => ({
    missingAnchors: [...document.querySelectorAll('a[href^="#"]')].filter(a => !document.getElementById(a.hash.slice(1))).map(a => a.hash),
    images: [...document.images].map(i => ({ loaded: i.complete && i.naturalWidth > 0, width: i.naturalWidth })),
    desktopOverflow: document.documentElement.scrollWidth > innerWidth,
  }));
  expect(result.missingAnchors).toEqual([]);
  expect(result.images.every(i => i.loaded)).toBe(true);
  expect(result.images.length).toBe(3);
  expect(result.desktopOverflow).toBe(false);
  const localLinks = await page.locator('a[href]').evaluateAll(links => links.map(a => a.href).filter(href => href.startsWith('file:')));
  result.missingLocalLinks = localLinks.filter(href => !existsSync(fileURLToPath(href)));
  const pendingIndex = pathToFileURL(resolve(folder, '../index.html')).href;
  expect(result.missingLocalLinks.filter(href => href !== pendingIndex)).toEqual([]);
  await page.screenshot({ path: `${folder}/desktop.png` });
  await page.locator('#map').screenshot({ path: `${folder}/map-desktop.png` });
  await page.locator('#configure').screenshot({ path: `${folder}/captures-desktop.png` });
  await page.locator('#G1').screenshot({ path: `${folder}/question-desktop.png` });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.evaluate(() => window.scrollTo({ top: 0, behavior: 'instant' }));
  result.mobileOverflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth);
  expect(result.mobileOverflow).toBe(false);
  await page.screenshot({ path: `${folder}/mobile.png` });
  await page.locator('#map').screenshot({ path: `${folder}/map-mobile.png` });
  await page.locator('#G1').screenshot({ path: `${folder}/question-mobile.png` });
  expect(requests).toEqual([]);
  Object.assign(result, { externalRequests: requests, javascript: 'disabled', desktop: '1440x1000', mobile: '390x844', checkedAt: new Date().toISOString() });
  await writeFile(`${folder}/verification.json`, JSON.stringify(result, null, 2) + '\n');
  console.log('H: desktop/mobile no overflow, 3 images loaded, anchors and local links checked, no external requests, JavaScript disabled.');
} finally { await browser.close(); }
