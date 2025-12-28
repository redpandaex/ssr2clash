import { Toaster } from "sonner";
import { ConversionPage } from "@/components/pages/ConversionPage";
import "./App.css";

function App() {
	return (
		<>
			<ConversionPage />
			<Toaster position="top-right" richColors />
		</>
	);
}

export default App;
