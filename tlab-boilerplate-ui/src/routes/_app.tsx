import { createFileRoute, redirect } from "@tanstack/react-router";
import { AppLayout } from "../features/app-shell/AppLayout";
import {
	authVerificationQuery,
	currentUserQuery,
} from "../features/auth/services/queries";

export const Route = createFileRoute("/_app")({
	beforeLoad: async ({ context }) => {
		const verified = await context.queryClient.fetchQuery(
			authVerificationQuery,
		);
		if (!verified) throw redirect({ to: "/login" });
		const user = await context.queryClient.fetchQuery(currentUserQuery);
		if (!user) throw redirect({ to: "/login" });
		return { user };
	},
	component: AppLayout,
});
