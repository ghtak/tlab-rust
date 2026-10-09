import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
	cleanup,
	fireEvent,
	render,
	screen,
	waitFor,
} from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { changeUserRoles, getRoles, getUserRoles } from "../services/api";
import { UserRolePage } from "./UserRolePage";

vi.mock("@tanstack/react-router", async (importOriginal) => ({
	...(await importOriginal<typeof import("@tanstack/react-router")>()),
	Link: ({ children }: { children: React.ReactNode }) => (
		<a href="/users">{children}</a>
	),
	useNavigate: () => vi.fn(),
}));

vi.mock("../services/api", () => ({
	changeUserRoles: vi.fn(),
	getRoles: vi.fn(),
	getUserRoles: vi.fn(),
}));

afterEach(() => {
	cleanup();
	vi.clearAllMocks();
});

test("페이지를 넘겨 선택한 롤의 추가와 해제를 저장한다", async () => {
	vi.mocked(getUserRoles).mockResolvedValue({
		user: {
			id: 7,
			name: "테스트 사용자",
			email: "test@example.com",
			status: "active",
		},
		roles: [{ id: 1, code: "admin", description: null }],
	});
	vi.mocked(getRoles).mockImplementation(async ({ page, pageSize }) => ({
		items:
			page === 1
				? [
						{ id: 1, code: "admin", description: null, permission_count: 1 },
						{ id: 2, code: "sales", description: null, permission_count: 0 },
					]
				: [{ id: 3, code: "reviewer", description: null, permission_count: 0 }],
		total: 21,
		page,
		page_size: pageSize,
	}));
	vi.mocked(changeUserRoles).mockResolvedValue();

	render(
		<QueryClientProvider client={new QueryClient()}>
			<UserRolePage userId={7} />
		</QueryClientProvider>,
	);

	await screen.findByRole("checkbox", { name: /sales/ });
	fireEvent.click(screen.getByRole("checkbox", { name: /sales/ }));
	fireEvent.click(screen.getByLabelText("다음 페이지"));
	await screen.findByRole("checkbox", { name: /reviewer/ });
	fireEvent.click(screen.getByRole("checkbox", { name: /reviewer/ }));
	fireEvent.click(screen.getByRole("button", { name: "연결됨" }));
	fireEvent.click(screen.getByRole("checkbox", { name: /admin/ }));
	fireEvent.click(screen.getByRole("button", { name: "저장" }));

	await waitFor(() =>
		expect(vi.mocked(changeUserRoles).mock.calls[0]?.[0]).toEqual({
			userId: 7,
			add_ids: [2, 3],
			remove_ids: [1],
		}),
	);
});
