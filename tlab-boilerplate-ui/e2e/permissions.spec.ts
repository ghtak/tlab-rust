import { expect, test } from "@playwright/test";

test("퍼미션 코드를 검색하면 목록이 바뀐다", async ({ page }) => {
	// 보호된 페이지에 진입할 수 있도록 현재 사용자 응답을 준비한다.
	await page.route("**/api/v1/auth/me", (route) =>
		route.fulfill({
			json: {
				data: {
					id: 1,
					name: "테스트 사용자",
					email: "test@example.com",
					status: "active",
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
	await expect(page.getByRole("cell", { name: "role:read" })).toBeVisible();

	await page
		.getByRole("searchbox", { name: "퍼미션 코드 검색" })
		.fill("user:manage");
	await page.getByRole("button", { name: "검색" }).click();

	await expect(page.getByRole("cell", { name: "user:manage" })).toBeVisible();
	await expect(page.getByRole("cell", { name: "role:read" })).toHaveCount(0);
});
