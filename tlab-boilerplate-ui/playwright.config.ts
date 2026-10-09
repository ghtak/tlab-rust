import { defineConfig } from "@playwright/test";

const baseURL = "http://127.0.0.1:3100";

export default defineConfig({
	testDir: "./e2e",
	use: { baseURL, browserName: "chromium" },
	webServer: {
		command: "node ./node_modules/vite/bin/vite.js --host 127.0.0.1 --port 3100 --strictPort",
		url: baseURL,
		reuseExistingServer: !process.env.CI,
	},
});
