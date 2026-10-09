import { createFileRoute } from "@tanstack/react-router";
import { RolePermissionPage } from "../features/access/pages/RolePermissionPage";

export const Route = createFileRoute("/_app/roles/$roleId/permissions")({
	component: RolePermissionRoute,
});

function RolePermissionRoute() {
	const { roleId } = Route.useParams();
	return <RolePermissionPage roleId={Number(roleId)} />;
}
