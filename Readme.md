# le_dns_online

## What is it ?

le_dns_online is a frontend intended to provide dns validation of *Let's Encrypt* for the french cloud provider & hoster [online.net](https://www.online.net/en). More specifically, its goal is to integrate easily with [acme.sh](https://github.com/Neilpang/acme.sh).

## How do I install it ?

You just need to add 'dns_online.sh' and the binary le_dns_online to the dnsapi folder inside '~/.acme.sh' (or whichever folder you use for acme.sh). You must then update the api_key in dns_online.sh to your private key (given at https://console.online.net/en/api/access) and you're good to go !

## How does it work ?

Acme.sh calls the fonction 'dns_online_add' from 'dns_online.sh', which calls le_dns_online binary.
le_dns_online then:
1) create a new temporary zone
2) copy the currently active zone to the temporary one
3) Add the record needed for *Let's Encrypt* AND the version of the active zone
4) Enable the temporary zone
Acme.sh takes back control again, and execute the authentification request. Subsequently, it calls 'dns_online_rm', which calls (again) le_dns_online binary.
This time, le_dns_online:
1) retrieve the version of the original zone
2) enable that zone (this is essentially a rollback of our changes)
3) delete the temporary zone

And voilà ! You have your certs validated ;)

## Known issues

Do NOT use this program concurrently !!!
This may break the ongoing validations (or worse, corrupt your DNS zone, event if it's quite unlikely).

## Can I contribute ?

Sure, go ahead ! Prepare youself to dig your way through some terrible Rust code, however ^_^
