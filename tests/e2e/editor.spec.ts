import { test, expect } from '@playwright/test';

test.describe('AgentScript Editor', () => {
  test('should load the editor page', async ({ page }) => {
    await page.goto('/');
    
    // Check that the page title is correct
    await expect(page).toHaveTitle(/AgentScript Editor Demo/);
    
    // Check that the header is present
    await expect(page.locator('h1')).toContainText('AgentScript Editor Demo');
  });

  test('should load WASM module successfully', async ({ page }) => {
    await page.goto('/');
    
    // Wait for WASM to load (check for the loaded badge)
    const wasmBadge = page.locator('.wasm-badge');
    await expect(wasmBadge).toBeVisible();
    
    // Wait for WASM to be loaded (the badge should have the 'loaded' class)
    await expect(wasmBadge).toHaveClass(/loaded/, { timeout: 10000 });
    await expect(wasmBadge).toContainText(/Ready|Loaded/i);
  });

  test('should have Monaco editor loaded', async ({ page }) => {
    await page.goto('/');
    
    // Wait for the editor container to be present
    const editorContainer = page.locator('#editor');
    await expect(editorContainer).toBeVisible({ timeout: 10000 });
    
    // Check that Monaco editor is initialized by checking for Monaco-specific elements
    await page.waitForFunction(() => {
      const editor = document.querySelector('#editor');
      // Monaco editor adds specific classes and children when initialized
      return editor && (
        editor.querySelector('.monaco-editor') !== null ||
        editor.querySelector('.view-lines') !== null ||
        editor.classList.contains('monaco-editor')
      );
    }, { timeout: 10000 });
  });

  test('should load example recipes in dropdown', async ({ page }) => {
    await page.goto('/');
    
    // Wait for the recipe selector to be present
    const recipeSelect = page.locator('select#recipeSelect');
    await expect(recipeSelect).toBeVisible();
    
    // Check that there are recipe options (more than just the default option)
    const options = await recipeSelect.locator('option').count();
    expect(options).toBeGreaterThan(1);
  });

  test('should have working buttons in header', async ({ page }) => {
    await page.goto('/');
    
    // Check that the Parse button is present
    const parseButton = page.locator('button').filter({ hasText: /parse/i });
    await expect(parseButton).toBeVisible();
    
    // Check that the Show Benchmark button is present
    const benchmarkButton = page.locator('button').filter({ hasText: /benchmark/i });
    await expect(benchmarkButton).toBeVisible();
  });

  test('should have outline panel', async ({ page }) => {
    await page.goto('/');
    
    // Wait for the outline panel to be present
    const outlinePanel = page.locator('.outline-panel, .panel').first();
    await expect(outlinePanel).toBeVisible();
  });

  test('should switch between recipes', async ({ page }) => {
    await page.goto('/');
    
    // Wait for WASM to load
    await expect(page.locator('.wasm-badge')).toHaveClass(/loaded/, { timeout: 10000 });
    
    // Select a recipe from dropdown
    const recipeSelect = page.locator('select#recipeSelect');
    await expect(recipeSelect).toBeVisible();
    
    const options = await recipeSelect.locator('option').all();
    if (options.length > 1) {
      // Get the initial value
      const initialValue = await recipeSelect.inputValue() || '';
      
      // Select the second option (first is usually "Select a recipe...")
      await recipeSelect.selectOption({ index: 1 });
      
      // Wait for the select value to change (indicating the recipe loaded)
      await page.waitForFunction(
        (oldValue: string) => {
          const select = document.querySelector('select#recipeSelect') as HTMLSelectElement | null;
          return select && select.value !== oldValue && select.value !== '';
        },
        initialValue,
        { timeout: 5000 }
      );
      
      // Verify Monaco editor has content by checking for view-lines with content
      await page.waitForFunction(() => {
        const viewLines = document.querySelector('.view-lines');
        return viewLines && viewLines.children.length > 0;
      }, { timeout: 5000 });
    }
  });
});
