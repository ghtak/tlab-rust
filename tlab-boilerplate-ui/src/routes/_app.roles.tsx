import { createFileRoute } from "@tanstack/react-router";
import { RolePage } from "../features/access/pages/RolePage";

export const Route = createFileRoute("/_app/roles")({
	component: RolePage,
});
