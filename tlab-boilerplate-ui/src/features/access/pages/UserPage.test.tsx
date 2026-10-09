import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
	cleanup,
	fireEvent,
	render,
	screen,
	waitFor,
} from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { changeUserStatus, getUsers } from "../services/api";
import { UserPage } from "./UserPage";

vi.mock("@tanstack/react-router", async (importOriginal) => ({
	...(await importOriginal<typeof import("@tanstack/react-router")>()),
	useNavigate: () => vi.fn(),
}));

vi.mock("../services/api", () => ({
	changeUserStatus: vi.fn(),
	getUsers: vi.fn(),
}));

afterEach(() => {
	cleanup();
	vi.clearAllMocks();
});

test("사용자 검색과 상태 필터, 페이지 이동 결과를 표시한다", async () => {
	vi.mocked(getUsers).mockImplementation(
		async ({ status, page, pageSize }) => ({
			items: [
				{
					id: page,
					name: page === 1 ? "김민준" : "이서연",
					email: page === 1 ? "minjun@example.com" : "seoyeon@example.com",
					status: status || "active",
					roles: page === 1 ? ["admin", "sales"] : [],
					providers: page === 1 ? ["google", "managed"] : [],
					latest_login_at: page === 1 ? "2025-01-01T00:00:00Z" : null,
				},
			],
			total: 21,
			page,
			page_size: pageSize,
		}),
	);

	render(
		<QueryClientProvider client={new QueryClient()}>
			<UserPage />
		</QueryClientProvider>,
	);

	await screen.findByText("김민준");
	expect(screen.getByText("admin, sales")).toBeTruthy();
	expect(screen.getByText("google, managed")).toBeTruthy();
	expect(screen.getByText(/2025/)).toBeTruthy();
	fireEvent.change(
		screen.getByRole("searchbox", { name: "이름 또는 이메일 검색" }),
		{
			target: { value: "  minjun  " },
		},
	);
	fireEvent.click(screen.getByRole("button", { name: "검색" }));
	await waitFor(() =>
		expect(getUsers).toHaveBeenLastCalledWith({
			q: "minjun",
			status: "",
			page: 1,
			pageSize: 20,
		}),
	);

	fireEvent.click(screen.getByRole("button", { name: "상태 필터" }));
	fireEvent.click(screen.getByRole("menuitem", { name: "활성" }));
	await waitFor(() =>
		expect(getUsers).toHaveBeenLastCalledWith({
			q: "minjun",
			status: "active",
			page: 1,
			pageSize: 20,
		}),
	);
	await screen.findByText("김민준");
	fireEvent.click(screen.getByLabelText("다음 페이지"));
	await screen.findByText("이서연");
	expect(getUsers).toHaveBeenLastCalledWith({
		q: "minjun",
		status: "active",
		page: 2,
		pageSize: 20,
	});
	expect(screen.getAllByText("—")).toHaveLength(2);
});

test("작업 메뉴에서 사용자 계정을 비활성화하고 활성화한다", async () => {
	let status: "active" | "suspended" = "active";
	vi.mocked(getUsers).mockImplementation(async ({ page, pageSize }) => ({
		items: [
			{
				id: 7,
				name: "테스트 사용자",
				email: "test@example.com",
				status,
				roles: [],
				providers: ["managed"],
				latest_login_at: null,
			},
		],
		total: 1,
		page,
		page_size: pageSize,
	}));
	vi.mocked(changeUserStatus).mockImplementation(async ({ status: next }) => {
		status = next;
	});

	render(
		<QueryClientProvider client={new QueryClient()}>
			<UserPage />
		</QueryClientProvider>,
	);

	await screen.findByText("테스트 사용자");
	fireEvent.click(
		screen.getByRole("button", { name: "test@example.com 작업" }),
	);
	fireEvent.click(screen.getByRole("menuitem", { name: "계정 비활성화" }));
	expect(
		screen.getByText(/현재 로그인 세션은 즉시 종료되지 않으며/),
	).toBeTruthy();
	fireEvent.click(screen.getByRole("button", { name: "비활성화" }));
	await waitFor(() =>
		expect(vi.mocked(changeUserStatus).mock.calls[0]?.[0]).toEqual({
			userId: 7,
			status: "suspended",
		}),
	);
	await screen.findByText("비활성");

	fireEvent.click(
		screen.getByRole("button", { name: "test@example.com 작업" }),
	);
	fireEvent.click(screen.getByRole("menuitem", { name: "계정 활성화" }));
	fireEvent.click(screen.getByRole("button", { name: "활성화" }));
	await waitFor(() =>
		expect(vi.mocked(changeUserStatus).mock.calls[1]?.[0]).toEqual({
			userId: 7,
			status: "active",
		}),
	);
});
