import { chromium, expect } from '@playwright/test';
import { mkdir, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';
const root=resolve('design/alternatives/round-4/section-03');
await mkdir(`${root}/proof`,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true});
try {
  const page=await browser.newPage({viewport:{width:1600,height:1000},javaScriptEnabled:false});
  const network=[];
  page.on('request',request=>{if(/^https?:/.test(request.url()))network.push(request.url());});
  await page.goto(pathToFileURL(`${root}/index.html`).href);
  const missing=await page.locator('a[href^="#"]').evaluateAll(nodes=>nodes.filter(node=>!document.getElementById(node.hash.slice(1))).map(node=>node.hash));
  expect(missing).toEqual([]);
  const links=await page.locator('a[href]').evaluateAll(nodes=>nodes.map(node=>node.href).filter(href=>href.startsWith('file:')));
  expect(links.filter(href=>!existsSync(fileURLToPath(href)))).toEqual([]);
  const sizes=[];
  for(const width of [1600,1440,390]) {
    await page.setViewportSize({width,height:1000});
    const overflow=await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth);
    expect(overflow).toBe(false);
    sizes.push({width,overflow});
    await page.evaluate(()=>window.scrollTo({top:0,behavior:'instant'}));
    await page.screenshot({path:`${root}/proof/page-${width}.png`});
  }
  await page.setViewportSize({width:1600,height:1000});
  for(const id of ['A','B','C'])await page.locator(`#${id} .example`).screenshot({path:`${root}/proof/${id}.png`});
  expect(network).toEqual([]);
  await writeFile(`${root}/proof/verification.json`,JSON.stringify({sizes,missingAnchors:missing,externalRequests:network,javascript:'disabled',checkedAt:new Date().toISOString()},null,2)+'\n');
  console.log('Section alternatives: 1600/1440/390 widths, links and no-script/offline checks passed.');
}finally{await browser.close();}
