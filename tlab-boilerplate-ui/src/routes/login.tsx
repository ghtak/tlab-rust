import { createFileRoute, redirect } from "@tanstack/react-router";
import { LoginPage } from "../features/auth/pages/LoginPage";
import {
	authVerificationQuery,
	currentUserQuery,
} from "../features/auth/services/queries";

export const Route = createFileRoute("/login")({
	beforeLoad: async ({ context }) => {
		const verified = await context.queryClient.fetchQuery(authVerificationQuery);
		if (
			verified &&
			context.queryClient.getQueryData(currentUserQuery.queryKey) !== null
		)
			throw redirect({ to: "/" });
	},
	component: LoginPage,
});
