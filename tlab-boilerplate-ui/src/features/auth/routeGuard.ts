import { redirect } from "@tanstack/react-router";
import { hasPermission } from "./access";
import type { CurrentUser } from "./types";

export function requirePermission(user: CurrentUser, permission: string): void {
	if (!hasPermission(user, permission)) throw redirect({ to: "/forbidden" });
}
