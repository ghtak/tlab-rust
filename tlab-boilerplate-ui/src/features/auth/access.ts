import type { CurrentUser } from "./types";

export const pageAccess = {
	permissions: "access:manage",
	roles: "access:manage",
	users: "access:manage",
} as const;

export function hasPermission(
	user: CurrentUser | null,
	permission: string,
): boolean {
	return user?.permissions.includes(permission) ?? false;
}

export function hasAnyPermission(
	user: CurrentUser | null,
	permissions: string[],
): boolean {
	return (
		user !== null &&
		permissions.some((permission) => hasPermission(user, permission))
	);
}

export function hasAllPermissions(
	user: CurrentUser | null,
	permissions: string[],
): boolean {
	return (
		user !== null &&
		permissions.every((permission) => hasPermission(user, permission))
	);
}
