import { createFileRoute } from "@tanstack/react-router";
import { AccessDeniedPage } from "../features/auth/pages/AccessDeniedPage";

export const Route = createFileRoute("/_app/forbidden")({
	component: AccessDeniedPage,
});
