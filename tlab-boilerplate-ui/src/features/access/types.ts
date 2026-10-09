export type Permission = {
	id: number;
	code: string;
	description: string | null;
};

export type PermissionListParams = {
	code: string;
	page: number;
	pageSize: number;
};

export type PermissionList = {
	items: Permission[];
	total: number;
	page: number;
	page_size: number;
};

export type CreatePermissionInput = {
	code: string;
	description: string | null;
};

export type Role = {
	id: number;
	code: string;
	description: string | null;
	permission_count: number;
};

export type RoleListParams = {
	code: string;
	page: number;
	pageSize: number;
};

export type RoleList = {
	items: Role[];
	total: number;
	page: number;
	page_size: number;
};

export type CreateRoleInput = {
	code: string;
	description: string | null;
};

export type RolePermissions = {
	role: Role;
	permission_ids: number[];
	linked_permissions: Permission[];
};

export type ChangeRolePermissionsInput = {
	roleId: number;
	add_ids: number[];
	remove_ids: number[];
};
