import { Github, Linkedin, Server } from "lucide-react";

const DOH_SERVER_URL = process.env.NEXT_PUBLIC_CLIENT_URL ?? "https://doh.adityanithariya.com";
const GITHUB_URL = "https://github.com/adityanithariya/dns-resolver";
// TODO: swap in your actual LinkedIn profile URL.
const LINKEDIN_URL = "https://www.linkedin.com/in/adityanithariya";

export default function Footer() {
    return (
        <footer className="mx-auto mt-16 max-w-3xl border-t border-border px-6 py-6">
            <div className="flex flex-wrap items-center justify-between gap-3 text-xs text-muted-foreground">
                <a
                    href={DOH_SERVER_URL}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex items-center gap-1.5 hover:text-primary"
                >
                    <Server className="h-3.5 w-3.5" />
                    {DOH_SERVER_URL.replace(/^https?:\/\//, "")}
                </a>

                <div className="flex items-center gap-4">
                    <a
                        href={GITHUB_URL}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="flex items-center gap-1.5 hover:text-primary"
                    >
                        <Github className="h-3.5 w-3.5" />
                        GitHub
                    </a>
                    <a
                        href={LINKEDIN_URL}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="flex items-center gap-1.5 hover:text-primary"
                    >
                        <Linkedin className="h-3.5 w-3.5" />
                        LinkedIn
                    </a>
                </div>
            </div>
        </footer>
    );
}