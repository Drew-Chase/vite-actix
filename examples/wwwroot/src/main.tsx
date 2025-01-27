import React from "react";
import {BrowserRouter, Route, Routes, useNavigate} from "react-router-dom";
import ReactDOM from "react-dom/client";
import $ from "jquery";

import "./assets/css/index.css";
import Home from "./assets/pages/Home.tsx";
import About from "./assets/pages/About.tsx";
import Navigation from "./assets/components/Navigation.tsx";
import {ThemeProvider} from "./assets/providers/ThemeProvider.tsx";
import {HeroUIProvider} from "@heroui/react";


ReactDOM.createRoot($("#root")[0]!).render(
    <React.StrictMode>
        <BrowserRouter>
            <ThemeProvider>
                <MainContentRenderer/>
            </ThemeProvider>
        </BrowserRouter>
    </React.StrictMode>
);

export function MainContentRenderer()
{
    const navigate = useNavigate();
    return (
        <HeroUIProvider navigate={navigate}>
            <Navigation/>
            <Routes>
                <Route>
                    <Route path="/" element={<Home/>}/>
                    <Route path="/about" element={<About/>}/>
                </Route>
            </Routes>
        </HeroUIProvider>
    );
}
