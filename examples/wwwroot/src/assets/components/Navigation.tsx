import React from "react";
import {Link, Navbar, NavbarBrand, NavbarContent, NavbarItem, NavbarMenu, NavbarMenuItem, NavbarMenuToggle} from "@heroui/react";
import {ThemeSwitchComponent} from "../providers/ThemeProvider.tsx";

export default function Navigation()
{

    const [isMenuOpen, setIsMenuOpen] = React.useState(false);
    const pages = {
        "Home": "/",
        "About": "/about"
    };
    const menuItems = Object.keys(pages).map((item, index) =>
    {
        const url = Object.values(pages)[index];
        const isCurrentPage = window.location.pathname === url;
        return (
            <NavbarMenuItem key={`${item}-${index}`}>
                <Link href={url} color={isCurrentPage ? "primary" : "foreground"} aria-current="page" size="lg" className="w-full">
                    {item}
                </Link>
            </NavbarMenuItem>
        );
    });


    return (
        <Navbar onMenuOpenChange={setIsMenuOpen}>
            <NavbarContent>
                <NavbarMenuToggle aria-label={isMenuOpen ? "Close menu" : "Open menu"} className="sm:hidden"/>
                <NavbarBrand>
                    <p className="font-bold text-inherit">wwwroot</p>
                </NavbarBrand>
            </NavbarContent>

            <NavbarContent className="hidden sm:flex gap-4" justify="center">
                {menuItems}
            </NavbarContent>
            <NavbarContent justify="end">
                <NavbarItem>
                    <ThemeSwitchComponent/>
                </NavbarItem>
            </NavbarContent>

            <NavbarMenu>
                {menuItems}
            </NavbarMenu>
        </Navbar>);
}