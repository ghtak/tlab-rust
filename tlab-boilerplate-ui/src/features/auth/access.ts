import { redirect } from "@tanstack/react-router";
import type { CurrentUser } from "./types";

export const pageAccess = {
	permissions: "access:manage",
	roles: "access:manage",
	users: "access:manage",
} as const;

export function hasPermission(user: CurrentUser, permission: string): boolean {
	return user.permissions.includes(permission);
}

export function requirePermission(user: CurrentUser, permission: string): void {
	if (!hasPermission(user, permission)) throw redirect({ to: "/forbidden" });
}
