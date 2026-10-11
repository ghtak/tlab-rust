import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import ReactDOM from "react-dom/client";
import { AppIntlProvider } from "./lib/AppIntlProvider";
import { getRouter } from "./router";

const rootElement = document.getElementById("app");

if (!rootElement) {
	throw new Error("HTML에 'app' element가 존재하지 않습니다.");
}

const queryClient = new QueryClient();
const router = getRouter(queryClient);

if (!rootElement.innerHTML) {
	const root = ReactDOM.createRoot(rootElement);
	root.render(
		<QueryClientProvider client={queryClient}>
			<AppIntlProvider>
				<RouterProvider router={router} />
			</AppIntlProvider>
		</QueryClientProvider>,
	);
}
