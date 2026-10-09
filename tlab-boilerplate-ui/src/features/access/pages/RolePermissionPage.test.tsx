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
	changeRolePermissions,
	getPermissions,
	getRolePermissions,
} from "../services/api";
import { RolePermissionPage } from "./RolePermissionPage";

vi.mock("@tanstack/react-router", async (importOriginal) => ({
	...(await importOriginal<typeof import("@tanstack/react-router")>()),
	Link: ({ children }: { children: React.ReactNode }) => (
		<a href="/roles">{children}</a>
	),
	useNavigate: () => vi.fn(),
}));

vi.mock("../services/api", () => ({
	changeRolePermissions: vi.fn(),
	getPermissions: vi.fn(),
	getRolePermissions: vi.fn(),
}));

afterEach(() => {
	cleanup();
	vi.clearAllMocks();
});

test("페이지를 넘겨 선택한 퍼미션의 추가·해제 차이를 저장한다", async () => {
	vi.mocked(getRolePermissions).mockResolvedValue({
		role: { id: 7, code: "reviewer", description: null, permission_count: 1 },
		permission_ids: [1],
		linked_permissions: [{ id: 1, code: "role:read", description: null }],
	});
	vi.mocked(getPermissions).mockImplementation(async ({ page, pageSize }) => ({
		items:
			page === 1
				? [
						{ id: 1, code: "role:read", description: null },
						{ id: 2, code: "role:create", description: null },
					]
				: [{ id: 3, code: "role:delete", description: null }],
		total: 21,
		page,
		page_size: pageSize,
	}));
	vi.mocked(changeRolePermissions).mockResolvedValue();

	render(
		<QueryClientProvider client={new QueryClient()}>
			<RolePermissionPage roleId={7} />
		</QueryClientProvider>,
	);

	await waitFor(() => expect(getPermissions).toHaveBeenCalled());
	await screen.findByText("role:create");
	await screen.findByRole("checkbox", { name: /role:create/ });
	fireEvent.click(screen.getByRole("checkbox", { name: /role:create/ }));
	fireEvent.click(screen.getByLabelText("다음 페이지"));
	await screen.findByRole("checkbox", { name: /role:delete/ });
	fireEvent.click(screen.getByRole("checkbox", { name: /role:delete/ }));
	fireEvent.click(screen.getByRole("button", { name: "연결됨" }));
	await screen.findByRole("checkbox", { name: /role:read/ });
	fireEvent.click(screen.getByRole("checkbox", { name: /role:read/ }));
	expect(screen.getByText("변경: 추가 2개 · 해제 1개")).toBeTruthy();
	fireEvent.click(screen.getByRole("button", { name: "저장" }));
	await waitFor(() =>
		expect(vi.mocked(changeRolePermissions).mock.calls[0]?.[0]).toEqual({
			roleId: 7,
			add_ids: [2, 3],
			remove_ids: [1],
		}),
	);
});
