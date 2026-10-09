import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
	cleanup,
	fireEvent,
	render,
	screen,
	waitFor,
} from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import {
	createPermission,
	deletePermission,
	getPermissions,
} from "../services/api";
import { PermissionPage } from "./PermissionPage";

vi.mock("../services/api", () => ({
	createPermission: vi.fn(),
	deletePermission: vi.fn(),
	getPermissions: vi.fn(),
}));

afterEach(() => {
	cleanup();
	vi.clearAllMocks();
});

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

test("퍼미션 추가 후 목록을 다시 조회한다", async () => {
	let created = false;
	vi.mocked(getPermissions).mockImplementation(async ({ page, pageSize }) => ({
		items: created
			? [{ id: 2, code: "order:read", description: "주문 조회" }]
			: [],
		total: created ? 1 : 0,
		page,
		page_size: pageSize,
	}));
	vi.mocked(createPermission).mockImplementation(async (input) => {
		created = true;
		return { id: 2, ...input };
	});

	const queryClient = new QueryClient();
	render(
		<QueryClientProvider client={queryClient}>
			<PermissionPage />
		</QueryClientProvider>,
	);

	await screen.findByText("등록된 퍼미션이 없습니다.");
	fireEvent.click(screen.getByRole("button", { name: "퍼미션 추가" }));
	fireEvent.change(screen.getByRole("textbox", { name: "코드 *" }), {
		target: { value: "order:read" },
	});
	fireEvent.change(screen.getByRole("textbox", { name: "설명" }), {
		target: { value: "주문 조회" },
	});
	fireEvent.click(screen.getByRole("button", { name: "추가" }));

	await waitFor(() =>
		expect(vi.mocked(createPermission).mock.calls[0]?.[0]).toEqual({
			code: "order:read",
			description: "주문 조회",
		}),
	);
	await screen.findByText("order:read");
	expect(screen.queryByRole("dialog")).toBeNull();
});

test("행 메뉴에서 확인 후 퍼미션을 삭제하고 목록을 갱신한다", async () => {
	let deleted = false;
	vi.mocked(getPermissions).mockImplementation(async ({ page, pageSize }) => ({
		items: deleted ? [] : [{ id: 7, code: "role:read", description: null }],
		total: deleted ? 0 : 1,
		page,
		page_size: pageSize,
	}));
	vi.mocked(deletePermission).mockImplementation(async () => {
		deleted = true;
	});

	const queryClient = new QueryClient();
	render(
		<QueryClientProvider client={queryClient}>
			<PermissionPage />
		</QueryClientProvider>,
	);

	await screen.findByText("role:read");
	fireEvent.click(screen.getByRole("button", { name: "role:read 작업" }));
	fireEvent.click(screen.getByRole("menuitem", { name: "삭제" }));
	expect(screen.getByText(/모든 롤에서 연결이/)).toBeTruthy();
	fireEvent.click(screen.getByRole("button", { name: "삭제" }));

	await waitFor(() =>
		expect(vi.mocked(deletePermission).mock.calls[0]?.[0]).toBe(7),
	);
	await screen.findByText("등록된 퍼미션이 없습니다.");
});
