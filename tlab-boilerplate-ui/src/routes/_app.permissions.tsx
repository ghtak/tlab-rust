import { createFileRoute } from "@tanstack/react-router";
import { PermissionPage } from "../features/access/pages/PermissionPage";

export const Route = createFileRoute("/_app/permissions")({
	component: PermissionPage,
});
