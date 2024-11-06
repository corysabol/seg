import { z } from "zod";

export type PacketInfo = {
  listener_ip: string;
  network_tag: string;
  source_ip: string;
  source_port: number;
  target_port: number;
  protocol: "tcp" | "udp";
  flags: string[];
  timestamp: string;
};

export const PacketInfoSchema = z.object({
  listener_ip: z.string().ip({ version: "v4" }),
  network_tag: z.string(),
  source_ip: z.string().ip({ version: "v4" }),
  source_port: z.number().int().min(0).max(65535),
  target_port: z.number().int().min(0).max(65535),
  protocol: z.enum(["tcp", "udp"]),
  flags: z.array(z.string()),
  timestamp: z.string(),
});
