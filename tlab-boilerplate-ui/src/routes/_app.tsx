import { createFileRoute, redirect } from "@tanstack/react-router";
import { AppLayout } from "../features/app-shell/AppLayout";
import { currentUserQuery } from "../features/auth/services/queries";

export const Route = createFileRoute("/_app")({
	beforeLoad: async ({ context }) => {
		const user = await context.queryClient.query(currentUserQuery);
		if (!user) throw redirect({ to: "/login" });
		return { user };
	},
	component: AppRoute,
});

function AppRoute() {
	const { user } = Route.useRouteContext();
	return <AppLayout user={user} />;
}
