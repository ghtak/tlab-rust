import { createFileRoute, Outlet } from "@tanstack/react-router";
import { pageAccess, requirePermission } from "../features/auth/access";

export const Route = createFileRoute("/_app/roles")({
	beforeLoad: ({ context }) =>
		requirePermission(context.user, pageAccess.roles),
	component: Outlet,
});
