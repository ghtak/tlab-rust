import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { HTTPError } from "ky";
import {
	ChevronDownIcon,
	KeyRoundIcon,
	MoreHorizontalIcon,
	PowerIcon,
} from "lucide-react";
import { useState } from "react";
import { FormattedMessage, useIntl } from "react-intl";
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
} from "../../../components/ui/alert-dialog";
import { Button } from "../../../components/ui/button";
import { Card, CardHeader, CardTitle } from "../../../components/ui/card";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "../../../components/ui/dropdown-menu";
import { Input } from "../../../components/ui/input";
import {
	Pagination,
	PaginationContent,
	PaginationItem,
	PaginationLink,
	PaginationNext,
	PaginationPrevious,
} from "../../../components/ui/pagination";
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from "../../../components/ui/table";
import { changeUserStatus } from "../services/api";
import { usersQuery } from "../services/queries";
import type { User, UserStatus } from "../types";

const PAGE_SIZE = 20;

const statusStyles: Record<UserStatus, string> = {
	active: "bg-emerald-50 text-emerald-700 ring-emerald-200 dark:bg-emerald-950 dark:text-emerald-300 dark:ring-emerald-800",
	suspended: "bg-amber-50 text-amber-700 ring-amber-200 dark:bg-amber-950 dark:text-amber-300 dark:ring-amber-800",
	withdrawn: "bg-muted text-muted-foreground ring-border",
};

export function UserPage() {
	const intl = useIntl();
	const statusLabels: Record<UserStatus, string> = {
		active: intl.formatMessage({
			id: "access.user.status.active",
			defaultMessage: "활성",
		}),
		suspended: intl.formatMessage({
			id: "access.user.status.suspended",
			defaultMessage: "비활성",
		}),
		withdrawn: intl.formatMessage({
			id: "access.user.status.withdrawn",
			defaultMessage: "탈퇴",
		}),
	};
	const navigate = useNavigate();
	const queryClient = useQueryClient();
	const [inputQ, setInputQ] = useState("");
	const [q, setQ] = useState("");
	const [status, setStatus] = useState<UserStatus | "">("");
	const [page, setPage] = useState(1);
	const [userToChangeStatus, setUserToChangeStatus] = useState<User | null>(
		null,
	);
	const { data, error, isPending } = useQuery(
		usersQuery({ q, status, page, pageSize: PAGE_SIZE }),
	);
	const statusChange = useMutation({
		mutationFn: changeUserStatus,
		onSuccess: async () => {
			setUserToChangeStatus(null);
			await queryClient.invalidateQueries({ queryKey: ["users"] });
		},
	});
	const totalPages = Math.max(1, Math.ceil((data?.total ?? 0) / PAGE_SIZE));
	const firstVisiblePage = Math.max(1, Math.min(page - 2, totalPages - 4));
	const visiblePages = Array.from(
		{ length: Math.min(5, totalPages - firstVisiblePage + 1) },
		(_, index) => firstVisiblePage + index,
	);

	return (
		<div className="space-y-5">
			<div>
				<p className="mb-2 text-sm text-muted-foreground">
					<FormattedMessage
						id="common.accessManagement"
						defaultMessage="접근 관리"
					/>{" "}
					/{" "}
					<span className="text-foreground">
						<FormattedMessage id="common.users" defaultMessage="사용자" />
					</span>
				</p>
				<h1 className="text-3xl font-semibold tracking-tight">
					<FormattedMessage id="common.users" defaultMessage="사용자" />
				</h1>
				<p className="mt-1 text-sm text-muted-foreground">
					<FormattedMessage
						id="access.user.description"
						defaultMessage="사용자 계정의 상태와 연결된 롤을 조회합니다."
					/>
				</p>
			</div>

			<Card className="gap-0 bg-card py-0 shadow-sm">
				<CardHeader className="flex flex-wrap items-center justify-between gap-3 border-b border-border py-4 sm:px-6">
					<div className="flex items-center gap-3">
						<CardTitle>
							<FormattedMessage id="common.list" defaultMessage="목록" />
						</CardTitle>
						<span className="text-sm text-muted-foreground">
							{data ? (
								<FormattedMessage
									id="common.itemCount"
									defaultMessage="{count}개 항목"
									values={{ count: data.total }}
								/>
							) : (
								<FormattedMessage
									id="common.loadingItems"
									defaultMessage="항목 조회 중"
								/>
							)}
						</span>
					</div>
					<div className="ml-auto flex w-full flex-wrap items-center justify-end gap-2 sm:w-auto">
						<DropdownMenu>
							<DropdownMenuTrigger
								render={
									<Button
										variant="outline"
										className="min-w-28 justify-between"
									/>
								}
								aria-label={intl.formatMessage({
									id: "access.user.statusFilter",
									defaultMessage: "상태 필터",
								})}
							>
								{status
									? statusLabels[status]
									: intl.formatMessage({
											id: "access.user.allStatuses",
											defaultMessage: "전체 상태",
										})}
								<ChevronDownIcon />
							</DropdownMenuTrigger>
							<DropdownMenuContent align="end">
								{(
									[
										[
											"",
											intl.formatMessage({
												id: "access.user.allStatuses",
												defaultMessage: "전체 상태",
											}),
										],
										["active", statusLabels.active],
										["suspended", statusLabels.suspended],
										["withdrawn", statusLabels.withdrawn],
									] as const
								).map(([value, label]) => (
									<DropdownMenuItem
										key={value}
										onClick={() => {
											setStatus(value);
											setPage(1);
										}}
									>
										{label}
									</DropdownMenuItem>
								))}
							</DropdownMenuContent>
						</DropdownMenu>
						<form
							className="flex w-full gap-2 sm:w-80"
							onSubmit={(event) => {
								event.preventDefault();
								setQ(inputQ.trim());
								setPage(1);
							}}
						>
							<label htmlFor="user-search" className="sr-only">
								<FormattedMessage
									id="access.user.search"
									defaultMessage="이름 또는 이메일 검색"
								/>
							</label>
							<Input
								id="user-search"
								type="search"
								placeholder={intl.formatMessage({
									id: "access.user.search",
									defaultMessage: "이름 또는 이메일 검색",
								})}
								value={inputQ}
								onChange={(event) => setInputQ(event.target.value)}
							/>
							<Button type="submit">
								<FormattedMessage id="common.search" defaultMessage="검색" />
							</Button>
						</form>
					</div>
				</CardHeader>
				{isPending ? (
					<output className="block px-5 py-12 text-center text-sm text-muted-foreground">
						<FormattedMessage
							id="access.user.loading"
							defaultMessage="사용자를 불러오는 중입니다."
						/>
					</output>
				) : error ? (
					<p
						role="alert"
						className="px-5 py-12 text-center text-sm text-destructive"
					>
						{error instanceof HTTPError && error.response.status === 403 ? (
							<FormattedMessage
								id="access.user.forbidden"
								defaultMessage="사용자 목록을 볼 권한이 없습니다."
							/>
						) : (
							<FormattedMessage
								id="access.user.loadFailed"
								defaultMessage="사용자 목록을 불러오지 못했습니다."
							/>
						)}
					</p>
				) : data.items.length === 0 ? (
					<p className="px-5 py-12 text-center text-sm text-muted-foreground">
						{q || status ? (
							<FormattedMessage
								id="common.searchEmpty"
								defaultMessage="검색 결과가 없습니다."
							/>
						) : (
							<FormattedMessage
								id="access.user.empty"
								defaultMessage="등록된 사용자가 없습니다."
							/>
						)}
					</p>
				) : (
					<div className="overflow-x-auto">
						<Table className="min-w-[860px]">
							<TableHeader className="bg-muted/50 text-xs text-muted-foreground">
								<TableRow>
									<TableHead className="px-5 sm:px-6">
										<FormattedMessage
											id="access.user.name"
											defaultMessage="이름"
										/>
									</TableHead>
									<TableHead className="px-5 sm:px-6">
										<FormattedMessage
											id="access.user.email"
											defaultMessage="이메일"
										/>
									</TableHead>
									<TableHead className="px-5 sm:px-6">
										<FormattedMessage
											id="access.user.status"
											defaultMessage="상태"
										/>
									</TableHead>
									<TableHead className="px-5 sm:px-6">
										<FormattedMessage id="common.roles" defaultMessage="롤" />
									</TableHead>
									<TableHead className="px-5 sm:px-6">
										<FormattedMessage
											id="access.user.providers"
											defaultMessage="로그인 수단"
										/>
									</TableHead>
									<TableHead className="px-5 sm:px-6">
										<FormattedMessage
											id="access.user.lastLogin"
											defaultMessage="마지막 로그인"
										/>
									</TableHead>
									<TableHead className="w-16 px-5 text-right sm:px-6">
										<FormattedMessage
											id="common.actions"
											defaultMessage="작업"
										/>
									</TableHead>
								</TableRow>
							</TableHeader>
							<TableBody>
								{data.items.map((user) => (
									<TableRow key={user.id}>
										<TableCell className="px-5 py-4 font-medium sm:px-6">
											{user.name}
										</TableCell>
										<TableCell className="px-5 py-4 text-muted-foreground sm:px-6">
											{user.email}
										</TableCell>
										<TableCell className="px-5 py-4 sm:px-6">
											<span
												className={`inline-flex rounded-full px-2.5 py-0.5 text-xs font-medium ring-1 ring-inset ${statusStyles[user.status]}`}
											>
												{statusLabels[user.status]}
											</span>
										</TableCell>
										<TableCell className="px-5 py-4 text-muted-foreground sm:px-6">
											{user.roles.length > 0 ? user.roles.join(", ") : "—"}
										</TableCell>
										<TableCell className="px-5 py-4 text-muted-foreground sm:px-6">
											{user.providers.length > 0
												? user.providers.join(", ")
												: "—"}
										</TableCell>
										<TableCell className="px-5 py-4 text-muted-foreground sm:px-6">
											{user.latest_login_at
												? intl.formatDate(new Date(user.latest_login_at), {
														dateStyle: "medium",
														timeStyle: "short",
													})
												: intl.formatMessage({
														id: "access.user.noLogin",
														defaultMessage: "기록 없음",
													})}
										</TableCell>
										<TableCell className="px-5 py-4 text-right sm:px-6">
											<DropdownMenu>
												<DropdownMenuTrigger
													render={<Button variant="ghost" size="icon-sm" />}
													aria-label={`${user.email} ${intl.formatMessage({ id: "common.actions", defaultMessage: "작업" })}`}
												>
													<MoreHorizontalIcon />
												</DropdownMenuTrigger>
												<DropdownMenuContent align="end">
													<DropdownMenuItem
														onClick={() =>
															void navigate({
																to: "/users/$userId/roles",
																params: { userId: String(user.id) },
															})
														}
													>
														<KeyRoundIcon />{" "}
														<FormattedMessage
															id="access.user.manageRoles"
															defaultMessage="롤 관리"
														/>
													</DropdownMenuItem>
													{user.status !== "withdrawn" &&
														user.email !== "admin@localhost" && (
															<DropdownMenuItem
																onClick={() => {
																	statusChange.reset();
																	setUserToChangeStatus(user);
																}}
															>
																<PowerIcon />{" "}
																{user.status === "active" ? (
																	<FormattedMessage
																		id="access.user.suspend"
																		defaultMessage="계정 비활성화"
																	/>
																) : (
																	<FormattedMessage
																		id="access.user.activate"
																		defaultMessage="계정 활성화"
																	/>
																)}
															</DropdownMenuItem>
														)}
												</DropdownMenuContent>
											</DropdownMenu>
										</TableCell>
									</TableRow>
								))}
							</TableBody>
						</Table>
					</div>
				)}
				<div className="border-t border-border px-5 py-4 sm:px-6">
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
									aria-disabled={page <= 1 || isPending}
									tabIndex={page <= 1 || isPending ? -1 : undefined}
									onClick={(event) => {
										event.preventDefault();
										if (page > 1 && !isPending) setPage(page - 1);
									}}
								/>
							</PaginationItem>
							{visiblePages.map((visiblePage) => (
								<PaginationItem key={visiblePage}>
									<PaginationLink
										href="#"
										isActive={visiblePage === page}
										aria-label={intl.formatMessage(
											{ id: "common.page", defaultMessage: "{page} 페이지" },
											{ page: visiblePage },
										)}
										aria-disabled={isPending}
										tabIndex={isPending ? -1 : undefined}
										onClick={(event) => {
											event.preventDefault();
											if (!isPending) setPage(visiblePage);
										}}
									>
										{visiblePage}
									</PaginationLink>
								</PaginationItem>
							))}
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
									aria-disabled={!data || page >= totalPages || isPending}
									tabIndex={
										!data || page >= totalPages || isPending ? -1 : undefined
									}
									onClick={(event) => {
										event.preventDefault();
										if (data && page < totalPages && !isPending)
											setPage(page + 1);
									}}
								/>
							</PaginationItem>
						</PaginationContent>
					</Pagination>
				</div>
			</Card>
			<AlertDialog
				open={userToChangeStatus !== null}
				onOpenChange={(open) => {
					if (!open && !statusChange.isPending) setUserToChangeStatus(null);
				}}
			>
				<AlertDialogContent>
					<AlertDialogHeader>
						<AlertDialogTitle>
							{userToChangeStatus?.status === "active" ? (
								<FormattedMessage
									id="access.user.suspendConfirm"
									defaultMessage="계정을 비활성화할까요?"
								/>
							) : (
								<FormattedMessage
									id="access.user.activateConfirm"
									defaultMessage="계정을 활성화할까요?"
								/>
							)}
						</AlertDialogTitle>
						<AlertDialogDescription>
							<FormattedMessage
								id="access.user.changeDescription"
								defaultMessage="{email} 계정의 상태를 변경합니다."
								values={{ email: userToChangeStatus?.email }}
							/>
							{userToChangeStatus?.status === "active" && (
								<>
									{" "}
									<FormattedMessage
										id="access.user.suspendNote"
										defaultMessage="현재 로그인 세션은 즉시 종료되지 않으며, 다음 로그인부터 차단됩니다."
									/>
								</>
							)}
						</AlertDialogDescription>
					</AlertDialogHeader>
					{statusChange.isError && (
						<p role="alert" className="text-sm text-destructive">
							{statusChange.error instanceof HTTPError &&
							statusChange.error.response.status === 400 ? (
								<FormattedMessage
									id="access.user.changeBlocked"
									defaultMessage="현재 계정 상태에서는 변경할 수 없습니다. 목록을 새로고침해 주세요."
								/>
							) : (
								<FormattedMessage
									id="access.user.changeFailed"
									defaultMessage="계정 상태를 변경하지 못했습니다. 다시 시도해 주세요."
								/>
							)}
						</p>
					)}
					<AlertDialogFooter>
						<AlertDialogCancel disabled={statusChange.isPending} autoFocus>
							<FormattedMessage id="common.cancel" defaultMessage="취소" />
						</AlertDialogCancel>
						<AlertDialogAction
							variant={
								userToChangeStatus?.status === "active"
									? "destructive"
									: "default"
							}
							disabled={statusChange.isPending}
							onClick={() => {
								if (userToChangeStatus) {
									statusChange.mutate({
										userId: userToChangeStatus.id,
										status:
											userToChangeStatus.status === "active"
												? "suspended"
												: "active",
									});
								}
							}}
						>
							{statusChange.isPending ? (
								<FormattedMessage
									id="access.user.changing"
									defaultMessage="변경 중..."
								/>
							) : userToChangeStatus?.status === "active" ? (
								<FormattedMessage
									id="access.user.suspendAction"
									defaultMessage="비활성화"
								/>
							) : (
								<FormattedMessage
									id="access.user.activateAction"
									defaultMessage="활성화"
								/>
							)}
						</AlertDialogAction>
					</AlertDialogFooter>
				</AlertDialogContent>
			</AlertDialog>
		</div>
	);
}
