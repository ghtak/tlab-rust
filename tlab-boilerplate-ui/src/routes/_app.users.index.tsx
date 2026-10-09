import { createFileRoute } from "@tanstack/react-router";
import { UserPage } from "../features/access/pages/UserPage";

export const Route = createFileRoute("/_app/users/")({
	component: UserPage,
});
