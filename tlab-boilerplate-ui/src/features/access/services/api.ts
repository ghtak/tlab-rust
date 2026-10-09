import { api } from "../../../api";
import type { PermissionList, PermissionListParams } from "../types";

export async function getPermissions({
	code,
	page,
	pageSize,
}: PermissionListParams): Promise<PermissionList> {
	const response = await api
		.get("auth/permissions", {
			searchParams: {
				...(code ? { code } : {}),
				page: String(page),
				page_size: String(pageSize),
			},
		})
		.json<{ data: PermissionList }>();
	return response.data;
}
