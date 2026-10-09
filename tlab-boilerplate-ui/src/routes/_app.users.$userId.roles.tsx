import { createFileRoute } from "@tanstack/react-router";
import { UserRolePage } from "../features/access/pages/UserRolePage";

export const Route = createFileRoute("/_app/users/$userId/roles")({
	component: UserRoleRoute,
});

function UserRoleRoute() {
	const { userId } = Route.useParams();
	return <UserRolePage userId={Number(userId)} />;
}
