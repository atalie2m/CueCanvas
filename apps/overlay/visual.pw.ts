import { expect, test } from "@playwright/test";
import { PNG } from "pngjs";

test.beforeEach(async ({ page }) => {
  await page.route("**/assets/fixture-photo*", async (route) => {
    await route.fulfill({
      body: fixtureImagePng(),
      contentType: "image/png",
    });
  });
});

test("transparent background remains transparent", async ({ page }) => {
  await page.goto("/?fixture=transparent");
  const screenshot = await page.screenshot({ omitBackground: true });
  const png = PNG.sync.read(screenshot);
  expect(alphaAt(png, 320, 180)).toBe(0);
});

test("CJK text wraps inside its frame", async ({ page }) => {
  await page.goto("/?fixture=cjk-wrapping");
  const text = page.locator("[data-item-id='cjk-copy']");
  await expect(text).toBeVisible();
  await expect(text).toHaveJSProperty("clientWidth", 250);
  const contained = await text.evaluate(
    (element) =>
      element.scrollWidth <= element.clientWidth + 1 &&
      element.scrollHeight <= element.clientHeight + 1,
  );
  expect(contained).toBe(true);
});

test("font fallback stack is applied", async ({ page }) => {
  await page.goto("/?fixture=font-fallback");
  const fontFamily = await page
    .locator("[data-item-id='fallback-copy']")
    .evaluate((element) => getComputedStyle(element).fontFamily);
  expect(fontFamily).toContain("Definitely Missing CueCanvas Font");
  expect(fontFamily).toContain("system-ui");
});

test("overflowing text stays clipped to its frame", async ({ page }) => {
  await page.goto("/?fixture=text-overflow");
  const box = await page
    .locator("[data-item-id='overflow-copy']")
    .boundingBox();
  expect(box?.width).toBe(180);
  expect(box?.height).toBe(72);
  const overflowStyle = await page
    .locator("[data-item-id='overflow-copy']")
    .evaluate((element) => getComputedStyle(element).overflow);
  expect(overflowStyle).toBe("hidden");
});

test("image fit fixture renders nonblank pixels", async ({ page }) => {
  await page.goto("/?fixture=image-fit");
  await expect(page.locator("[data-item-id='fit-image']")).toBeVisible();
  const screenshot = await page.screenshot({ omitBackground: true });
  const png = PNG.sync.read(screenshot);
  expect(countOpaquePixels(png)).toBeGreaterThan(1_000);
});

test("clear fixture produces an empty overlay", async ({ page }) => {
  await page.goto("/?fixture=clear");
  await expect(page.locator("#overlay > *")).toHaveCount(0);
});

test("blackout fixture fills the canvas with black", async ({ page }) => {
  await page.goto("/?fixture=blackout");
  const screenshot = await page.screenshot({ omitBackground: true });
  const png = PNG.sync.read(screenshot);
  expect(rgbaAt(png, 320, 180)).toEqual([0, 0, 0, 255]);
});

test("reconnect recovery fixture renders after a blank state", async ({
  page,
}) => {
  await page.goto("/?fixture=reconnect-recovery");
  await expect(page.locator("[data-item-id='reconnect-ready']")).toBeVisible();
  const screenshot = await page.screenshot({ omitBackground: true });
  const png = PNG.sync.read(screenshot);
  expect(countOpaquePixels(png)).toBeGreaterThan(10_000);
});

function fixtureImagePng() {
  const png = new PNG({ width: 80, height: 80 });
  for (let y = 0; y < png.height; y += 1) {
    for (let x = 0; x < png.width; x += 1) {
      const offset = (png.width * y + x) << 2;
      png.data[offset] = x < 40 ? 239 : 37;
      png.data[offset + 1] = y < 40 ? 68 : 99;
      png.data[offset + 2] = x < 40 ? 68 : 235;
      png.data[offset + 3] = 255;
    }
  }
  return PNG.sync.write(png);
}

function alphaAt(png: PNG, x: number, y: number) {
  return png.data[(png.width * y + x) * 4 + 3];
}

function rgbaAt(png: PNG, x: number, y: number) {
  const offset = (png.width * y + x) * 4;
  return [
    png.data[offset],
    png.data[offset + 1],
    png.data[offset + 2],
    png.data[offset + 3],
  ];
}

function countOpaquePixels(png: PNG) {
  let count = 0;
  for (let offset = 3; offset < png.data.length; offset += 4) {
    if (png.data[offset] > 0) count += 1;
  }
  return count;
}
