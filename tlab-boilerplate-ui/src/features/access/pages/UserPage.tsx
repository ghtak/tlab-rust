import { useQuery } from "@tanstack/react-query";
import { HTTPError } from "ky";
import { ChevronDownIcon } from "lucide-react";
import { useState } from "react";
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
import { usersQuery } from "../services/queries";
import type { UserStatus } from "../types";

const PAGE_SIZE = 20;

const statusLabels: Record<UserStatus, string> = {
	active: "활성",
	suspended: "중지",
	withdrawn: "탈퇴",
};

export function UserPage() {
	const [inputQ, setInputQ] = useState("");
	const [q, setQ] = useState("");
	const [status, setStatus] = useState<UserStatus | "">("");
	const [page, setPage] = useState(1);
	const { data, error, isPending } = useQuery(
		usersQuery({ q, status, page, pageSize: PAGE_SIZE }),
	);
	const totalPages = Math.max(1, Math.ceil((data?.total ?? 0) / PAGE_SIZE));
	const firstVisiblePage = Math.max(1, Math.min(page - 2, totalPages - 4));
	const visiblePages = Array.from(
		{ length: Math.min(5, totalPages - firstVisiblePage + 1) },
		(_, index) => firstVisiblePage + index,
	);

	return (
		<div className="space-y-5">
			<div>
				<p className="mb-2 text-sm text-slate-500">
					접근 관리 / <span className="text-slate-900">사용자</span>
				</p>
				<h1 className="text-3xl font-semibold tracking-tight">사용자</h1>
				<p className="mt-1 text-sm text-slate-500">
					사용자 계정의 상태와 연결된 롤을 조회합니다.
				</p>
			</div>

			<Card className="gap-0 bg-white py-0 shadow-sm">
				<CardHeader className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-100 py-4 sm:px-6">
					<div className="flex items-center gap-3">
						<CardTitle>목록</CardTitle>
						<span className="text-sm text-slate-500">
							{data ? `${data.total}개 항목` : "항목 조회 중"}
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
								aria-label="상태 필터"
							>
								{status ? statusLabels[status] : "전체 상태"}
								<ChevronDownIcon />
							</DropdownMenuTrigger>
							<DropdownMenuContent align="end">
								{(
									[
										["", "전체 상태"],
										["active", "활성"],
										["suspended", "중지"],
										["withdrawn", "탈퇴"],
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
								이름 또는 이메일 검색
							</label>
							<Input
								id="user-search"
								type="search"
								placeholder="이름 또는 이메일 검색"
								value={inputQ}
								onChange={(event) => setInputQ(event.target.value)}
							/>
							<Button type="submit">검색</Button>
						</form>
					</div>
				</CardHeader>
				{isPending ? (
					<output className="block px-5 py-12 text-center text-sm text-slate-500">
						사용자를 불러오는 중입니다.
					</output>
				) : error ? (
					<p
						role="alert"
						className="px-5 py-12 text-center text-sm text-destructive"
					>
						{error instanceof HTTPError && error.response.status === 403
							? "사용자 목록을 볼 권한이 없습니다."
							: "사용자 목록을 불러오지 못했습니다."}
					</p>
				) : data.items.length === 0 ? (
					<p className="px-5 py-12 text-center text-sm text-slate-500">
						{q || status
							? "검색 결과가 없습니다."
							: "등록된 사용자가 없습니다."}
					</p>
				) : (
					<div className="overflow-x-auto">
						<Table className="min-w-[720px]">
							<TableHeader className="bg-slate-50 text-xs text-slate-500">
								<TableRow>
									<TableHead className="px-5 sm:px-6">이름</TableHead>
									<TableHead className="px-5 sm:px-6">이메일</TableHead>
									<TableHead className="px-5 sm:px-6">상태</TableHead>
									<TableHead className="px-5 sm:px-6">롤</TableHead>
									<TableHead className="px-5 sm:px-6">로그인 수단</TableHead>
								</TableRow>
							</TableHeader>
							<TableBody>
								{data.items.map((user) => (
									<TableRow key={user.id}>
										<TableCell className="px-5 py-4 font-medium sm:px-6">
											{user.name}
										</TableCell>
										<TableCell className="px-5 py-4 text-slate-600 sm:px-6">
											{user.email}
										</TableCell>
										<TableCell className="px-5 py-4 text-slate-600 sm:px-6">
											{statusLabels[user.status]}
										</TableCell>
										<TableCell className="px-5 py-4 text-slate-600 sm:px-6">
											{user.roles.length > 0 ? user.roles.join(", ") : "—"}
										</TableCell>
										<TableCell className="px-5 py-4 text-slate-600 sm:px-6">
											{user.providers.length > 0
												? user.providers.join(", ")
												: "—"}
										</TableCell>
									</TableRow>
								))}
							</TableBody>
						</Table>
					</div>
				)}
				<div className="border-t border-slate-100 px-5 py-4 sm:px-6">
					<Pagination>
						<PaginationContent>
							<PaginationItem>
								<PaginationPrevious
									href="#"
									text="이전"
									aria-label="이전 페이지"
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
										aria-label={`${visiblePage} 페이지`}
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
									text="다음"
									aria-label="다음 페이지"
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
		</div>
	);
}
