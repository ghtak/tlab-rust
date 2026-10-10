import { expect, test } from "@playwright/test";

test("로그인 상태는 verify로 확인하고 사용자 정보는 필요할 때 조회한다", async ({
	page,
}) => {
	let signedIn = false;
	let verifyRequests = 0;
	let meRequests = 0;

	await page.route("**/api/v1/auth/verify", (route) => {
		verifyRequests += 1;
		return route.fulfill({ status: signedIn ? 200 : 401, json: {} });
	});
	await page.route("**/api/v1/auth/me", (route) => {
		meRequests += 1;
		return route.fulfill({
			json: {
				data: {
					id: 1,
					name: "테스트 사용자",
					email: "test@example.com",
					status: "active",
					permissions: ["access:manage"],
				},
			},
		});
	});
	await page.route("**/api/v1/auth/login", (route) => {
		signedIn = true;
		return route.fulfill({ json: {} });
	});

	await page.goto("/login");
	await expect(page.getByRole("heading", { name: "로그인" })).toBeVisible();
	expect(verifyRequests).toBeGreaterThan(0);
	expect(meRequests).toBe(0);

	await page.getByLabel("이메일").fill("test@example.com");
	await page.getByLabel("비밀번호").fill("password");
	await page.getByRole("button", { name: "로그인", exact: true }).click();
	await expect(page).toHaveURL(/\/$/);
	await expect(page.getByText("테스트 사용자", { exact: true })).toBeVisible();
	expect(meRequests).toBe(1);

	const callsAfterLogin = verifyRequests;
	await page.locator('a[href="/permissions"]').first().click();
	await expect(page).toHaveURL(/\/permissions$/);
	expect(verifyRequests).toBeGreaterThan(callsAfterLogin);
	expect(meRequests).toBe(1);

	await page.reload();
	await expect(page.getByText("테스트 사용자", { exact: true })).toBeVisible();
	expect(meRequests).toBe(2);
});

test("verify 성공 후 me가 401이면 로그인 화면에 머문다", async ({ page }) => {
	await page.route("**/api/v1/auth/verify", (route) =>
		route.fulfill({ json: {} }),
	);
	await page.route("**/api/v1/auth/me", (route) =>
		route.fulfill({ status: 401, json: {} }),
	);

	await page.goto("/");
	await expect(page).toHaveURL(/\/login$/);
	await expect(page.getByRole("heading", { name: "로그인" })).toBeVisible();
});
