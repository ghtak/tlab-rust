import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useNavigate, useRouter } from "@tanstack/react-router";
import { HTTPError } from "ky";
import { useEffect, useState } from "react";
import { FormattedMessage, useIntl } from "react-intl";
import { Button } from "../../../components/ui/button";
import { Card, CardHeader, CardTitle } from "../../../components/ui/card";
import { Checkbox } from "../../../components/ui/checkbox";
import { Input } from "../../../components/ui/input";
import {
	Pagination,
	PaginationContent,
	PaginationItem,
	PaginationLink,
	PaginationNext,
	PaginationPrevious,
} from "../../../components/ui/pagination";
import { currentUserQuery } from "../../auth/services/queries";
import { changeUserRoles } from "../services/api";
import { rolesQuery, userRolesQuery } from "../services/queries";
import type { Role, UserRoles } from "../types";

const PAGE_SIZE = 20;

export function UserRolePage({ userId }: { userId: number }) {
	const validUserId = Number.isSafeInteger(userId) && userId > 0;
	const { data, error, isPending } = useQuery({
		...userRolesQuery(userId),
		enabled: validUserId,
	});

	if (!validUserId)
		return (
			<p role="alert">
				<FormattedMessage
					id="access.userRoles.invalidAddress"
					defaultMessage="잘못된 사용자 주소입니다."
				/>
			</p>
		);
	if (isPending)
		return (
			<p>
				<FormattedMessage
					id="access.userRoles.loading"
					defaultMessage="사용자의 롤을 불러오는 중입니다."
				/>
			</p>
		);
	if (error) {
		return (
			<p role="alert">
				{error instanceof HTTPError && error.response.status === 404 ? (
					<FormattedMessage
						id="access.userRoles.notFound"
						defaultMessage="사용자를 찾을 수 없습니다."
					/>
				) : (
					<FormattedMessage
						id="access.userRoles.loadFailed"
						defaultMessage="사용자의 롤을 불러오지 못했습니다."
					/>
				)}
			</p>
		);
	}
	return <UserRoleEditor key={userId} initial={data} />;
}

function UserRoleEditor({ initial }: { initial: UserRoles }) {
	const intl = useIntl();
	const queryClient = useQueryClient();
	const navigate = useNavigate();
	const router = useRouter();
	const [inputCode, setInputCode] = useState("");
	const [code, setCode] = useState("");
	const [page, setPage] = useState(1);
	const [filter, setFilter] = useState<"all" | "linked">("all");
	const [selected, setSelected] = useState(
		() => new Set(initial.roles.map((role) => role.id)),
	);
	const [known, setKnown] = useState(
		() => new Map(initial.roles.map((role) => [role.id, role])),
	);
	const catalog = useQuery({
		...rolesQuery({ code, page, pageSize: PAGE_SIZE }),
		enabled: filter === "all",
	});
	useEffect(() => {
		if (!catalog.data) return;
		setKnown((current) => {
			const next = new Map(current);
			for (const role of catalog.data.items) next.set(role.id, role);
			return next;
		});
	}, [catalog.data]);
	const saved = new Set(initial.roles.map((role) => role.id));
	const addIds = [...selected].filter((id) => !saved.has(id));
	const removeIds = [...saved].filter((id) => !selected.has(id));
	const save = useMutation({
		mutationFn: changeUserRoles,
		onSuccess: async () => {
			await Promise.all([
				queryClient.invalidateQueries({ queryKey: ["users"] }),
				queryClient.invalidateQueries({
					queryKey: ["userRoles", initial.user.id],
				}),
				queryClient.invalidateQueries({ queryKey: currentUserQuery.queryKey }),
			]);
			await router.invalidate();
			void navigate({ to: "/users" });
		},
	});
	const linkedItems = [...selected]
		.map((id) => known.get(id))
		.filter((role): role is Role => Boolean(role))
		.filter((role) => role.code.toLowerCase().includes(code.toLowerCase()))
		.sort((a, b) => a.code.localeCompare(b.code));
	const items = filter === "linked" ? linkedItems : (catalog.data?.items ?? []);
	const totalPages = Math.max(
		1,
		Math.ceil((catalog.data?.total ?? 0) / PAGE_SIZE),
	);

	return (
		<div className="space-y-5">
			<div>
				<p className="mb-2 text-sm text-slate-500">
					<FormattedMessage
						id="common.accessManagement"
						defaultMessage="접근 관리"
					/>{" "}
					/{" "}
					<Link to="/users" className="hover:underline">
						<FormattedMessage id="common.users" defaultMessage="사용자" />
					</Link>{" "}
					/ {initial.user.name} /{" "}
					<FormattedMessage
						id="access.user.manageRoles"
						defaultMessage="롤 관리"
					/>
				</p>
				<h1 className="text-3xl font-semibold tracking-tight">
					{initial.user.name}{" "}
					<FormattedMessage
						id="access.user.manageRoles"
						defaultMessage="롤 관리"
					/>
				</h1>
				<p className="mt-1 text-sm text-slate-500">
					<FormattedMessage
						id="access.userRoles.connected"
						defaultMessage="{email} · 현재 {count}개 롤이 연결되었습니다."
						values={{ email: initial.user.email, count: selected.size }}
					/>
				</p>
			</div>
			<Card className="gap-0 bg-white py-0 shadow-sm">
				<CardHeader className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-100 py-4 sm:px-6">
					<div className="flex items-center gap-3">
						<CardTitle>
							<FormattedMessage
								id="access.userRoles.select"
								defaultMessage="롤 선택"
							/>
						</CardTitle>
						<Button
							variant={filter === "all" ? "secondary" : "ghost"}
							size="sm"
							aria-pressed={filter === "all"}
							onClick={() => setFilter("all")}
						>
							<FormattedMessage id="common.all" defaultMessage="전체" />
						</Button>
						<Button
							variant={filter === "linked" ? "secondary" : "ghost"}
							size="sm"
							aria-pressed={filter === "linked"}
							onClick={() => setFilter("linked")}
						>
							<FormattedMessage id="common.linked" defaultMessage="연결됨" />
						</Button>
					</div>
					<form
						className="ml-auto flex w-full gap-2 sm:w-80"
						onSubmit={(event) => {
							event.preventDefault();
							setCode(inputCode.trim());
							setPage(1);
						}}
					>
						<label htmlFor="user-role-code" className="sr-only">
							<FormattedMessage
								id="access.role.search"
								defaultMessage="롤 코드 검색"
							/>
						</label>
						<Input
							id="user-role-code"
							type="search"
							placeholder={intl.formatMessage({
								id: "common.codeSearch",
								defaultMessage: "코드 검색",
							})}
							value={inputCode}
							onChange={(event) => setInputCode(event.target.value)}
						/>
						<Button type="submit">
							<FormattedMessage id="common.search" defaultMessage="검색" />
						</Button>
					</form>
				</CardHeader>
				{filter === "all" && catalog.isPending ? (
					<output className="block px-5 py-12 text-center text-sm text-slate-500">
						<FormattedMessage
							id="access.role.loading"
							defaultMessage="롤을 불러오는 중입니다."
						/>
					</output>
				) : filter === "all" && catalog.error ? (
					<p
						role="alert"
						className="px-5 py-12 text-center text-sm text-destructive"
					>
						<FormattedMessage
							id="access.role.loadFailed"
							defaultMessage="롤 목록을 불러오지 못했습니다."
						/>
					</p>
				) : items.length === 0 ? (
					<p className="px-5 py-12 text-center text-sm text-slate-500">
						<FormattedMessage
							id="access.userRoles.empty"
							defaultMessage="표시할 롤이 없습니다."
						/>
					</p>
				) : (
					<div className="divide-y divide-slate-100">
						{items.map((role) => {
							const protectedRole =
								initial.user.email === "admin@localhost" &&
								role.code === "admin" &&
								saved.has(role.id);
							return (
								<div
									key={role.id}
									className="flex items-center gap-3 px-5 py-3 sm:px-6"
								>
									<Checkbox
										id={`user-role-${role.id}`}
										checked={selected.has(role.id)}
										disabled={protectedRole || save.isPending}
										onCheckedChange={(checked) => {
											setSelected((current) => {
												const next = new Set(current);
												if (checked) next.add(role.id);
												else next.delete(role.id);
												return next;
											});
										}}
									/>
									<label
										htmlFor={`user-role-${role.id}`}
										className="min-w-0 flex-1 cursor-pointer"
									>
										<span className="block font-medium">{role.code}</span>
										<span className="block text-sm text-slate-500">
											{protectedRole ? (
												<FormattedMessage
													id="access.userRoles.protected"
													defaultMessage="초기 관리자 계정에서 해제할 수 없습니다."
												/>
											) : (
												(role.description ??
												intl.formatMessage({
													id: "common.noDescription",
													defaultMessage: "설명 없음",
												}))
											)}
										</span>
									</label>
								</div>
							);
						})}
					</div>
				)}
				{filter === "all" && (
					<div className="border-t border-slate-100 px-5 py-4 sm:px-6">
						<Pagination>
							<PaginationContent>
								<PaginationItem>
									<PaginationPrevious
										href="#"
										text={intl.formatMessage({
											id: "common.previous",
											defaultMessage: "이전",
										})}
										aria-label={intl.formatMessage({
											id: "common.previousPage",
											defaultMessage: "이전 페이지",
										})}
										aria-disabled={page <= 1 || catalog.isPending}
										tabIndex={page <= 1 || catalog.isPending ? -1 : undefined}
										onClick={(event) => {
											event.preventDefault();
											if (page > 1 && !catalog.isPending) setPage(page - 1);
										}}
									/>
								</PaginationItem>
								<PaginationItem>
									<PaginationLink
										href="#"
										isActive
										aria-label={intl.formatMessage(
											{ id: "common.page", defaultMessage: "{page} 페이지" },
											{ page },
										)}
										onClick={(event) => event.preventDefault()}
									>
										{page}
									</PaginationLink>
								</PaginationItem>
								<PaginationItem>
									<PaginationNext
										href="#"
										text={intl.formatMessage({
											id: "common.next",
											defaultMessage: "다음",
										})}
										aria-label={intl.formatMessage({
											id: "common.nextPage",
											defaultMessage: "다음 페이지",
										})}
										aria-disabled={page >= totalPages || catalog.isPending}
										tabIndex={
											page >= totalPages || catalog.isPending ? -1 : undefined
										}
										onClick={(event) => {
											event.preventDefault();
											if (page < totalPages && !catalog.isPending)
												setPage(page + 1);
										}}
									/>
								</PaginationItem>
							</PaginationContent>
						</Pagination>
					</div>
				)}
			</Card>
			<div className="flex flex-wrap items-center justify-between gap-3">
				<p className="text-sm text-slate-600">
					<FormattedMessage
						id="common.changeSummary"
						defaultMessage="변경: 추가 {added}개 · 해제 {removed}개"
						values={{ added: addIds.length, removed: removeIds.length }}
					/>
				</p>
				<div className="flex gap-2">
					<Button
						variant="outline"
						disabled={save.isPending}
						onClick={() => void navigate({ to: "/users" })}
					>
						<FormattedMessage id="common.cancel" defaultMessage="취소" />
					</Button>
					<Button
						disabled={
							save.isPending || (addIds.length === 0 && removeIds.length === 0)
						}
						onClick={() =>
							save.mutate({
								userId: initial.user.id,
								role_ids: [...selected],
							})
						}
					>
						{save.isPending ? (
							<FormattedMessage
								id="common.saving"
								defaultMessage="저장 중..."
							/>
						) : (
							<FormattedMessage id="common.save" defaultMessage="저장" />
						)}
					</Button>
				</div>
			</div>
			{save.isError && (
				<p role="alert" className="text-sm text-destructive">
					{save.error instanceof HTTPError &&
					save.error.response.status === 400 ? (
						<FormattedMessage
							id="access.userRoles.invalidSelection"
							defaultMessage="선택한 롤과 변경 내용을 확인해 주세요."
						/>
					) : (
						<FormattedMessage
							id="access.userRoles.saveFailed"
							defaultMessage="변경 사항을 저장하지 못했습니다. 다시 시도해 주세요."
						/>
					)}
				</p>
			)}
		</div>
	);
}
