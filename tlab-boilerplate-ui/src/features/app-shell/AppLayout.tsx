import {
	Link,
	Outlet,
	useNavigate,
	useRouterState,
} from "@tanstack/react-router";
import {
	ChevronDown,
	KeyRound,
	LayoutDashboard,
	LogOut,
	Menu,
	ShieldCheck,
	Users,
	X,
} from "lucide-react";
import { useState } from "react";
import { FormattedMessage, useIntl } from "react-intl";
import { LocaleSelect } from "../../components/LocaleSelect";
import { ThemeSelect } from "../../components/ThemeSelect";
import { Button } from "../../components/ui/button";
import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from "../../components/ui/collapsible";
import { pageAccess } from "../auth/access";
import { useCurrentUser } from "../auth/hooks/useCurrentUser";
import { useLogout } from "../auth/hooks/useLogout";
import { usePermission } from "../auth/hooks/usePermission";

const accessMenuItems = [
	{
		to: "/permissions",
		message: { id: "common.permissions", defaultMessage: "퍼미션" },
		icon: KeyRound,
		permission: pageAccess.permissions,
	},
	{
		to: "/roles",
		message: { id: "common.roles", defaultMessage: "롤" },
		icon: ShieldCheck,
		permission: pageAccess.roles,
	},
	{
		to: "/users",
		message: { id: "common.users", defaultMessage: "사용자" },
		icon: Users,
		permission: pageAccess.users,
	},
] as const;

export function AppLayout() {
	const intl = useIntl();
	const pathname = useRouterState({
		select: (state) => state.location.pathname,
	});
	const navigate = useNavigate();
	const { user } = useCurrentUser();
	const { hasPermission } = usePermission();
	const signOut = useLogout();
	const [menuOpen, setMenuOpen] = useState(false);
	const visibleAccessMenuItems = accessMenuItems.filter(({ permission }) =>
		hasPermission(permission),
	);
	const accessMenuVisible = visibleAccessMenuItems.length > 0;

	function handleLogout() {
		signOut.mutate(undefined, {
			onSuccess: () => {
				void navigate({ to: "/login" });
			},
		});
	}

	if (!user) return null;

	return (
		<div className="min-h-screen bg-background text-foreground">
			<header className="sticky top-0 z-30 flex h-16 items-center justify-between border-b border-border bg-card px-4 sm:px-6">
				<div className="flex items-center gap-3">
					<Button
						variant="ghost"
						size="icon"
						className="md:hidden"
						aria-label={intl.formatMessage({
							id: "shell.openMenu",
							defaultMessage: "메뉴 열기",
						})}
						onClick={() => setMenuOpen(true)}
					>
						<Menu aria-hidden="true" />
					</Button>
					<Link
						to="/"
						className="flex items-center gap-2.5 font-semibold tracking-tight"
					>
						<span className="flex size-8 items-center justify-center rounded-lg bg-primary text-sm font-bold text-primary-foreground">
							t
						</span>
						<span>tlab</span>
					</Link>
					<span className="hidden h-5 w-px bg-border sm:block" />
					<span className="hidden text-sm text-muted-foreground sm:block">
						<FormattedMessage id="shell.console" defaultMessage="관리 콘솔" />
					</span>
				</div>
				<div className="flex items-center gap-3">
					<LocaleSelect />
					<ThemeSelect />
					<div className="hidden text-right sm:block">
						<p className="text-sm font-medium leading-5">{user.name}</p>
						<p className="max-w-48 truncate text-xs text-muted-foreground">
							{user.email}
						</p>
					</div>
					<span
						className="flex size-9 items-center justify-center rounded-full bg-muted text-sm font-semibold text-foreground"
						aria-hidden="true"
					>
						{user.name.charAt(0)}
					</span>
					<Button
						variant="ghost"
						size="icon"
						aria-label={intl.formatMessage({
							id: "common.logout",
							defaultMessage: "로그아웃",
						})}
						title={intl.formatMessage({
							id: "common.logout",
							defaultMessage: "로그아웃",
						})}
						onClick={handleLogout}
						disabled={signOut.isPending}
					>
						<LogOut aria-hidden="true" />
					</Button>
				</div>
			</header>

			{menuOpen && (
				<button
					type="button"
					className="fixed inset-0 z-40 bg-black/50 md:hidden"
					aria-label={intl.formatMessage({
						id: "shell.closeMenu",
						defaultMessage: "메뉴 닫기",
					})}
					onClick={() => setMenuOpen(false)}
				/>
			)}
			<aside
				className={`fixed bottom-0 left-0 top-16 z-50 w-60 border-r border-border bg-card transition-transform md:translate-x-0 ${menuOpen ? "translate-x-0" : "-translate-x-full"}`}
			>
				<nav
					aria-label={intl.formatMessage({
						id: "shell.mainMenu",
						defaultMessage: "주 메뉴",
					})}
					className="p-4"
				>
					<div className="mb-4 flex items-center justify-end md:hidden">
						<Button
							variant="ghost"
							size="icon-sm"
							aria-label={intl.formatMessage({
								id: "shell.closeMenu",
								defaultMessage: "메뉴 닫기",
							})}
							onClick={() => setMenuOpen(false)}
						>
							<X aria-hidden="true" />
						</Button>
					</div>
					<div className="space-y-1">
						<Link
							to="/"
							onClick={() => setMenuOpen(false)}
							aria-current={pathname === "/" ? "page" : undefined}
							className={`flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-colors ${pathname === "/" ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-muted hover:text-foreground"}`}
						>
							<LayoutDashboard className="size-4" aria-hidden="true" />
							<FormattedMessage
								id="common.dashboard"
								defaultMessage="대시보드"
							/>
						</Link>
						{accessMenuVisible && (
							<Collapsible
								key={pathname}
								defaultOpen={visibleAccessMenuItems.some(
									({ to }) => to === pathname,
								)}
							>
								<CollapsibleTrigger className="group flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left text-sm font-medium text-muted-foreground outline-none transition-colors hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring">
									<ShieldCheck className="size-4" aria-hidden="true" />
									<span className="flex-1">
										<FormattedMessage
											id="common.accessManagement"
											defaultMessage="접근 관리"
										/>
									</span>
									<ChevronDown
										className="size-4 transition-transform group-data-[open]:rotate-180"
										aria-hidden="true"
									/>
								</CollapsibleTrigger>
								<CollapsibleContent>
									<div className="ml-5 mt-1 space-y-1 border-l border-border pl-3">
										{visibleAccessMenuItems.map(
											({ to, message, icon: Icon }) => (
												<Link
													key={to}
													to={to}
													onClick={() => setMenuOpen(false)}
													aria-current={pathname === to ? "page" : undefined}
													className={`flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors ${pathname === to ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-muted hover:text-foreground"}`}
												>
													<Icon className="size-4" aria-hidden="true" />
													{intl.formatMessage(message)}
												</Link>
											),
										)}
									</div>
								</CollapsibleContent>
							</Collapsible>
						)}
					</div>
				</nav>
				<div className="absolute bottom-0 w-full border-t border-border p-4 text-xs text-muted-foreground">
					tlab workspace
				</div>
			</aside>

			<main className="min-h-[calc(100vh-4rem)] md:ml-60">
				<div className="mx-auto max-w-7xl px-4 py-8 sm:px-8 lg:py-10">
					{signOut.isError && (
						<p
							role="alert"
							className="mb-5 rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive"
						>
							<FormattedMessage
								id="shell.logoutFailed"
								defaultMessage="로그아웃하지 못했습니다. 다시 시도해 주세요."
							/>
						</p>
					)}
					<Outlet />
				</div>
			</main>
		</div>
	);
}
