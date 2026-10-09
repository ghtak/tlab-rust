import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { getPermissions } from "../services/api";
import { PermissionPage } from "./PermissionPage";

vi.mock("../services/api", () => ({ getPermissions: vi.fn() }));

// 1. `fireEvent.change(...)`가 검색창에 `user:manage`를 입력합니다. 이때는 입력 상태인 `inputCode`만 바뀌고 아직 조회하지 않습니다.
// 2. `fireEvent.click(... "검색")`이 검색 버튼을 누릅니다. 폼 제출 코드가 `code`를 `user:manage`로 바꿉니다.
// 3. `code`가 바뀌어 `useQuery`의 쿼리 키가 달라지고, 모의 `getPermissions`가 다시 호출됩니다.
// 4. `waitFor(...toHaveBeenLastCalledWith(...))`는 **`code: "user:manage", page: 1, pageSize: 20`으로 조회했는지** 확인합니다.
// 5. `findByText("user:manage")`는 새 결과가 표에 나타났는지 기다립니다. 모의 API가 전달받은 `code`를 항목 코드로 돌려주기 때문에 이 텍스트가 표시됩니다.
// 6. `queryByText("role:read")`가 `null`인지 확인해 이전 결과가 화면에서 사라졌는지 검사합니다.

test("코드 검색 시 해당 조건으로 퍼미션 목록을 다시 조회한다", async () => {
	vi.mocked(getPermissions).mockImplementation(
		async ({ code, page, pageSize }) => ({
			items: [
				{
					id: 1,
					code: code || "role:read",
					description: null,
				},
			],
			total: 1,
			page,
			page_size: pageSize,
		}),
	);

	const queryClient = new QueryClient();
	render(
		<QueryClientProvider client={queryClient}>
			<PermissionPage />
		</QueryClientProvider>,
	);

	await screen.findByText("role:read");
	fireEvent.change(
		screen.getByRole("searchbox", { name: "퍼미션 코드 검색" }),
		{
			target: { value: "user:manage" },
		},
	);
	fireEvent.click(screen.getByRole("button", { name: "검색" }));

	await waitFor(() =>
		expect(getPermissions).toHaveBeenLastCalledWith({
			code: "user:manage",
			page: 1,
			pageSize: 20,
		}),
	);
	await screen.findByText("user:manage");
	expect(screen.queryByText("role:read")).toBeNull();
});
