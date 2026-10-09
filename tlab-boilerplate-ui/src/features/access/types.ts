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
};

export type RoleWithPermissionCount = Role & {
	permission_count: number;
};

export type RoleListParams = {
	code: string;
	page: number;
	pageSize: number;
};

export type RoleList = {
	items: RoleWithPermissionCount[];
	total: number;
	page: number;
	page_size: number;
};

export type CreateRoleInput = {
	code: string;
	description: string | null;
};

export type RolePermissions = {
	role: RoleWithPermissionCount;
	permission_ids: number[];
	linked_permissions: Permission[];
};

export type ChangeRolePermissionsInput = {
	roleId: number;
	add_ids: number[];
	remove_ids: number[];
};

export type UserStatus = "active" | "suspended" | "withdrawn";

export type User = {
	id: number;
	name: string;
	email: string;
	status: UserStatus;
	roles: string[];
	providers: string[];
	latest_login_at: string | null;
};

export type ChangeUserStatusInput = {
	userId: number;
	status: "active" | "suspended";
};

export type UserListParams = {
	q: string;
	status: UserStatus | "";
	page: number;
	pageSize: number;
};

export type UserList = {
	items: User[];
	total: number;
	page: number;
	page_size: number;
};

export type UserRoles = {
	user: Pick<User, "id" | "name" | "email" | "status">;
	roles: Role[];
};

export type ChangeUserRolesInput = {
	userId: number;
	role_ids: number[];
};
