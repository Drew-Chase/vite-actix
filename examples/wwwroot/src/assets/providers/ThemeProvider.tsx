import {createContext, Dispatch, ReactNode, SetStateAction, useContext, useEffect, useState} from "react";
import $ from "jquery";
import {Button, Tooltip} from "@heroui/react";
import {Icon} from "@iconify-icon/react";

export enum Themes
{
    LIGHT = "light",
    DARK = "dark",
    SYSTEM = "system"
}

interface ThemeContextType
{
    theme: Themes;
    setTheme: Dispatch<SetStateAction<Themes>>;
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

export function ThemeProvider({children}: { children: ReactNode })
{
    const [theme, setTheme] = useState<Themes>(getSavedTheme());

    useEffect(() =>
    {
        const tmp = theme == Themes.SYSTEM ? getSystemTheme() : theme;
        $("html")
            .removeClass("dark")
            .removeClass("light")
            .addClass(tmp === Themes.DARK ? "dark" : "light");
        localStorage.setItem("app-theme", theme.toString());
    }, [theme]);

    return (
        <ThemeContext.Provider value={{theme, setTheme}}>
            {children}
        </ThemeContext.Provider>
    );
}

export function useTheme(): ThemeContextType
{
    const context = useContext(ThemeContext);
    if (!context)
    {
        throw new Error("useTheme must be used within a ThemeProvider");
    }
    return context;
}


function getSavedTheme(): Themes
{
    return localStorage.getItem("app-theme") as Themes | null || Themes.SYSTEM;
}

export function getSystemTheme(): Themes
{
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? Themes.DARK : Themes.LIGHT;
}

export function getRealTheme(theme: Themes): Themes
{
    return theme == Themes.SYSTEM ? getSystemTheme() : theme;
}

export function ThemeSwitchComponent()
{
    const {theme, setTheme} = useTheme();
    return (
        <Tooltip
            content={`Enable ${getRealTheme(theme) === "dark" ? "Light" : "Dark"} Mode`}>
            <Button
                variant={"light"}
                className={"min-w-0 h-[2rem] text-tiny"}
                radius={"sm"}
                onPress={() => setTheme(prev => getRealTheme(prev) === "dark" ? Themes.LIGHT : Themes.DARK)}
            >
                <Icon icon={`mage:${getRealTheme(theme) === "dark" ? "moon" : "sun"}-fill`} height="1rem"/>
            </Button>
        </Tooltip>
    );
}