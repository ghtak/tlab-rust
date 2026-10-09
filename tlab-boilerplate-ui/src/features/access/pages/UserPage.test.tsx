import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
	cleanup,
	fireEvent,
	render,
	screen,
	waitFor,
} from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { getUsers } from "../services/api";
import { UserPage } from "./UserPage";

vi.mock("../services/api", () => ({ getUsers: vi.fn() }));

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
