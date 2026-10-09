import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
	cleanup,
	fireEvent,
	render,
	screen,
	waitFor,
} from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { createRole, deleteRole, getRoles } from "../services/api";
import { RolePage } from "./RolePage";

vi.mock("../services/api", () => ({
	createRole: vi.fn(),
	deleteRole: vi.fn(),
	getRoles: vi.fn(),
}));

afterEach(() => {
	cleanup();
	vi.clearAllMocks();
});

test("롤 추가와 삭제 후 목록을 다시 조회한다", async () => {
	let role: {
		id: number;
		code: string;
		description: string | null;
		permission_count: number;
	} | null = null;
	vi.mocked(getRoles).mockImplementation(async ({ page, pageSize }) => ({
		items: role ? [role] : [],
		total: role ? 1 : 0,
		page,
		page_size: pageSize,
	}));
	vi.mocked(createRole).mockImplementation(async (input) => {
		role = { id: 7, permission_count: 0, ...input };
		return role;
	});
	vi.mocked(deleteRole).mockImplementation(async () => {
		role = null;
	});

	render(
		<QueryClientProvider client={new QueryClient()}>
			<RolePage />
		</QueryClientProvider>,
	);

	await screen.findByText("등록된 롤이 없습니다.");
	fireEvent.click(screen.getByRole("button", { name: "롤 추가" }));
	fireEvent.change(screen.getByRole("textbox", { name: "코드 *" }), {
		target: { value: "reviewer" },
	});
	fireEvent.click(screen.getByRole("button", { name: "추가" }));
	await screen.findByText("reviewer");
	expect(vi.mocked(createRole).mock.calls[0]?.[0]).toEqual({
		code: "reviewer",
		description: null,
	});

	fireEvent.click(screen.getByRole("button", { name: "reviewer 작업" }));
	fireEvent.click(screen.getByRole("menuitem", { name: "삭제" }));
	expect(screen.getByText(/연결된 퍼미션도 해제됩니다/)).toBeTruthy();
	fireEvent.click(screen.getByRole("button", { name: "삭제" }));
	await waitFor(() => expect(vi.mocked(deleteRole).mock.calls[0]?.[0]).toBe(7));
	await screen.findByText("등록된 롤이 없습니다.");
});
