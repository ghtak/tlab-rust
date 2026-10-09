import { createFileRoute, redirect } from "@tanstack/react-router";
import { LoginPage } from "../features/auth/pages/LoginPage";
import { currentUserQuery } from "../features/auth/services/queries";

export const Route = createFileRoute("/login")({
	beforeLoad: async ({ context }) => {
		const user = await context.queryClient.query(currentUserQuery);
		if (user) throw redirect({ to: "/" });
	},
	component: LoginPage,
});
