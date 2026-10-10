import { expect, test } from "@playwright/test";

test("퍼미션 코드를 검색하면 목록이 바뀐다", async ({ page }) => {
	// 보호된 페이지에 진입할 수 있도록 현재 사용자 응답을 준비한다.
	await page.route("**/api/v1/auth/verify", (route) =>
		route.fulfill({ json: {} }),
	);
	await page.route("**/api/v1/auth/me", (route) =>
		route.fulfill({
			json: {
				data: {
					id: 1,
					name: "테스트 사용자",
					email: "test@example.com",
					status: "active",
					permissions: ["access:manage"],
				},
			},
		}),
	);

	// 검색 요청의 code 값에 따라 다른 목록을 반환한다.
	await page.route("**/api/v1/auth/permissions*", (route) => {
		const searchCode = new URL(route.request().url()).searchParams.get("code");
		return route.fulfill({
			json: {
				data: {
					items: [
						{
							id: 1,
							code: searchCode ?? "role:read",
							description: null,
						},
					],
					total: 1,
					page: 1,
					page_size: 20,
				},
			},
		});
	});

	await page.goto("/permissions");
	await expect(page.getByRole("cell", { name: "role:read", exact: true })).toBeVisible();

	await page
		.getByRole("searchbox", { name: "퍼미션 코드 검색" })
		.fill("access:manage");
	await page.getByRole("button", { name: "검색" }).click();

	await expect(page.getByRole("cell", { name: "access:manage", exact: true })).toBeVisible();
	await expect(page.getByRole("cell", { name: "role:read", exact: true })).toHaveCount(0);
});

test("권한이 없으면 메뉴와 바로가기를 숨기고 직접 접근을 차단한다", async ({ page }) => {
	await page.route("**/api/v1/auth/verify", (route) =>
		route.fulfill({ json: {} }),
	);
	await page.route("**/api/v1/auth/me", (route) =>
		route.fulfill({
			json: {
				data: {
					id: 2,
					name: "일반 사용자",
					email: "user@example.com",
					status: "active",
					permissions: [],
				},
			},
		}),
	);

	await page.goto("/");
	await expect(page.getByRole("navigation").getByText("접근 관리")).toHaveCount(0);
	await expect(page.locator('a[href="/permissions"]')).toHaveCount(0);
	await expect(page.locator('a[href="/roles"]')).toHaveCount(0);
	await expect(page.locator('a[href="/users"]')).toHaveCount(0);

	for (const path of [
		"/permissions",
		"/roles/7/permissions",
		"/users/7/roles",
	]) {
		await page.goto(path);
		await expect(page).toHaveURL(/\/forbidden$/);
		await expect(page.getByRole("heading", { name: "접근 권한이 없습니다." })).toBeVisible();
	}
});
