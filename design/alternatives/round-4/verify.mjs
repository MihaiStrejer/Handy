import { chromium, expect } from "@playwright/test";
import { existsSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";

const root = resolve("design/alternatives/round-4");
await mkdir(`${root}/proof`, { recursive: true });
const browser = await chromium.launch({ channel: "msedge", headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1920, height: 1000 }, javaScriptEnabled: false });
  page.setDefaultTimeout(5000);
  const network = [];
  page.on("request", request => { if (/^https?:/.test(request.url())) network.push(request.url()); });
  await page.goto(pathToFileURL(`${root}/index.html`).href);
  const result = await page.evaluate(() => {
    const hero = document.querySelector('.hero');
    const style = getComputedStyle(hero);
    const subtitle = document.querySelector('#after .chapter-head p');
    return {
      contentWidth: hero.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight),
      subtitleWidth: subtitle.getBoundingClientRect().width,
      headingSize: getComputedStyle(document.querySelector('h1')).fontSize,
      pills: document.querySelectorAll('.pill').length,
      question: document.querySelector('#G1 h2').textContent,
      extraQuestionTitle: document.querySelector('#G1 .gap-card > h3') !== null,
      diagramConnectorStyles: [...document.querySelectorAll('.diagram .path,.diagram .connector')].map(path => getComputedStyle(path).strokeDasharray),
      imagesLoaded: [...document.images].every(image => image.complete && image.naturalWidth > 0),
      missingAnchors: [...document.querySelectorAll('a[href^="#"]')].filter(link => !document.getElementById(link.hash.slice(1))).map(link => link.hash),
      squareSamples: ['.request','.message pre','.reply-flow pre','.gap-card','.option'].map(selector => ({selector, radius:getComputedStyle(document.querySelector(selector)).borderRadius})),
      outputHeading: document.querySelector('#after h2').textContent,
      separatePersistence: document.querySelector('#next-recording .lifetime') !== null && document.querySelector('#after .lifetime') === null,
      sidebarWidth: document.querySelector('#contents').getBoundingClientRect().width,
      centeredWithinPane: Math.abs((hero.getBoundingClientRect().left + hero.getBoundingClientRect().right) / 2 - (document.querySelector('#content-pane').getBoundingClientRect().left + document.querySelector('#content-pane').clientWidth / 2)) < 1,
    };
  });
  expect(result.contentWidth).toBe(1440);
  expect(result.subtitleWidth).toBe(1440);
  expect(result.pills).toBe(0);
  expect(result.question).toBe('G1. What evidence permits automatic learning?');
  expect(result.extraQuestionTitle).toBe(false);
  expect(result.diagramConnectorStyles.every(value => value === 'none')).toBe(true);
  expect(result.imagesLoaded).toBe(true);
  expect(result.missingAnchors).toEqual([]);
  expect(result.squareSamples.every(item => item.radius === '0px')).toBe(true);
  expect(result.outputHeading).toBe('Follow the reply through Handy');
  expect(result.separatePersistence).toBe(true);
  expect(result.sidebarWidth).toBe(200);
  expect(result.centeredWithinPane).toBe(true);
  const links = await page.locator('a[href]').evaluateAll(nodes => nodes.map(node => node.href).filter(href => href.startsWith('file:')));
  expect(links.filter(href => !existsSync(fileURLToPath(href)))).toEqual([]);
  result.widths = [];
  for (const width of [1920, 1688, 1440, 1200, 900, 390]) {
    await page.setViewportSize({ width, height: 1000 });
    await page.evaluate(() => document.querySelector('#content-pane').scrollTo({top:0, behavior:'instant'}));
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth > innerWidth || document.querySelector('#content-pane').scrollWidth > document.querySelector('#content-pane').clientWidth);
    expect(overflow).toBe(false);
    result.widths.push({width, overflow});
    await page.screenshot({path:`${root}/proof/page-${width}.png`});
  }
  await page.setViewportSize({ width: 1920, height: 1000 });
  for (const [name, selector] of Object.entries({overview:'#overview', question:'#G1', request:'.request', response:'#after', persistence:'#next-recording'})) {
    await page.locator(selector).first().screenshot({path:`${root}/proof/${name}.png`});
  }
  await page.locator('#after').evaluate(element => element.scrollIntoView({block:'start',behavior:'instant'}));
  await page.screenshot({path:`${root}/proof/after-width.png`});
  const summary = page.locator('#contents summary');
  await summary.focus();
  await page.keyboard.press('Enter');
  await expect(page.locator('#contents')).not.toHaveAttribute('open');
  expect(await page.locator('#contents').evaluate(node=>node.getBoundingClientRect().width)).toBe(48);
  const collapsedWidth = await page.locator('.hero').evaluate(node=>node.clientWidth-parseFloat(getComputedStyle(node).paddingLeft)-parseFloat(getComputedStyle(node).paddingRight));
  expect(collapsedWidth).toBe(1440);
  await page.screenshot({path:`${root}/proof/sidebar-collapsed.png`});
  await page.keyboard.press('Enter');
  await expect(page.locator('#contents')).toHaveAttribute('open');
  await page.locator('.contents-list a[href="#after"]').click();
  await expect.poll(()=>page.locator('#after').evaluate(node=>Math.abs(node.getBoundingClientRect().top-20))).toBeLessThan(2);
  await page.setViewportSize({width:1920,height:400});
  const independent = await page.evaluate(()=>{
    const pane=document.querySelector('#content-pane'), nav=document.querySelector('.contents-list');
    pane.style.scrollBehavior='auto';
    pane.scrollTop=500;
    const paneBefore=pane.scrollTop;
    nav.scrollTop=150;
    const navAfter=nav.scrollTop;
    const contentUnchanged=pane.scrollTop===paneBefore;
    pane.scrollTop=900;
    return {navScrollable:nav.scrollHeight>nav.clientHeight,navMoved:navAfter>0,contentUnchanged,navUnchanged:nav.scrollTop===navAfter,bodyStayedPut:window.scrollY===0};
  });
  expect(Object.values(independent).every(Boolean)).toBe(true);
  result.sidebar={keyboardToggle:true,collapsedWidth:48,collapsedContentWidth:collapsedWidth,independent};
  const mobile=await browser.newPage({viewport:{width:390,height:844}});
  await mobile.goto(pathToFileURL(`${root}/index.html`).href);
  await expect(mobile.locator('#contents')).not.toHaveAttribute('open');
  await mobile.screenshot({path:`${root}/proof/sidebar-mobile.png`});
  await mobile.locator('#contents summary').click();
  await expect(mobile.locator('#contents')).toHaveAttribute('open');
  await mobile.screenshot({path:`${root}/proof/sidebar-mobile-expanded.png`});
  await mobile.close();
  expect(network).toEqual([]);
  Object.assign(result, {externalRequests:network, javascript:'disabled',checkedAt:new Date().toISOString()});
  await writeFile(`${root}/proof/verification.json`, JSON.stringify(result,null,2)+'\n');
  console.log('Passed: 1440px content/subtitle, restrained heading, no pills, solid connectors, square containers, G1 hierarchy, links/images and responsive widths.');
} finally {
  await browser.close();
}
