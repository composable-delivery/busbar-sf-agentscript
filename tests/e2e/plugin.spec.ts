import { test, expect } from '@playwright/test';

test.describe('Plugin Documentation Page', () => {
  test('should load the plugin documentation page', async ({ page }) => {
    await page.goto('/plugin.html');
    
    // Check that the page title is correct
    await expect(page).toHaveTitle(/AgentScript Parser Plugin/);
  });

  test('should have main heading', async ({ page }) => {
    await page.goto('/plugin.html');
    
    // Check that the main heading is present
    const heading = page.locator('h1').first();
    await expect(heading).toContainText(/AgentScript Parser Plugin|Salesforce CLI Plugin/i);
  });

  test('should have documentation content', async ({ page }) => {
    await page.goto('/plugin.html');
    
    // Check that there is documentation content (paragraphs or sections)
    const content = page.locator('body');
    await expect(content).toContainText(/plugin|install|command/i);
  });
});
