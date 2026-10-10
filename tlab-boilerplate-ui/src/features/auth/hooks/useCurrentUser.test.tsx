import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, expect, test, vi } from "vitest";
import { getCurrentUser } from "../services/api";
import { currentUserQuery } from "../services/queries";
import type { CurrentUser } from "../types";
import { useCurrentUser } from "./useCurrentUser";
import { usePermission } from "./usePermission";

vi.mock("../services/api", () => ({
	getCurrentUser: vi.fn(),
	verifyAuth: vi.fn(),
}));

afterEach(() => {
	cleanup();
	vi.clearAllMocks();
});

function withCurrentUser(user: CurrentUser | null) {
	const client = new QueryClient();
	client.setQueryData(currentUserQuery.queryKey, user);
	return ({ children }: { children: ReactNode }) => (
		<QueryClientProvider client={client}>{children}</QueryClientProvider>
	);
}

test("현재 사용자 쿼리의 정보를 훅에서 공유한다", () => {
	const user = {
		id: 1,
		name: "테스트 사용자",
		email: "test@example.com",
		status: "active",
		permissions: ["access:manage", "user:read"],
	};
	const { result } = renderHook(() => useCurrentUser(), {
		wrapper: withCurrentUser(user),
	});

	expect(result.current.user).toEqual(user);
	expect(result.current.permissions).toEqual(user.permissions);
	expect(result.current.isLoading).toBe(false);
});

test("권한 훅은 단일·일부·전체 권한을 구분한다", () => {
	const { result } = renderHook(() => usePermission(), {
		wrapper: withCurrentUser({
			id: 1,
			name: "테스트 사용자",
			email: "test@example.com",
			status: "active",
			permissions: ["user:read", "role:read"],
		}),
	});

	expect(result.current.hasPermission("user:read")).toBe(true);
	expect(result.current.hasPermission("user:write")).toBe(false);
	expect(result.current.hasAnyPermission(["user:write", "role:read"])).toBe(
		true,
	);
	expect(result.current.hasAllPermissions(["user:read", "role:read"])).toBe(
		true,
	);
	expect(result.current.hasAllPermissions(["user:read", "user:write"])).toBe(
		false,
	);
	expect(result.current.hasAllPermissions([])).toBe(true);
});

test("사용자가 없으면 빈 권한 목록도 허용하지 않는다", () => {
	const { result } = renderHook(() => usePermission(), {
		wrapper: withCurrentUser(null),
	});

	expect(result.current.hasPermission("user:read")).toBe(false);
	expect(result.current.hasAnyPermission(["user:read"])).toBe(false);
	expect(result.current.hasAllPermissions([])).toBe(false);
});

test("사용자를 불러오는 동안에는 권한을 허용하지 않는다", () => {
	vi.mocked(getCurrentUser).mockImplementation(() => new Promise(() => {}));
	const client = new QueryClient();
	const { result } = renderHook(() => usePermission(), {
		wrapper: ({ children }: { children: ReactNode }) => (
			<QueryClientProvider client={client}>{children}</QueryClientProvider>
		),
	});

	expect(result.current.isLoading).toBe(true);
	expect(result.current.hasPermission("user:read")).toBe(false);
	expect(result.current.hasAllPermissions([])).toBe(false);
});
