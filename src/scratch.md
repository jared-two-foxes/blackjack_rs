# Journal

The idea of this file is to be a dumping ground of my thoughts and ideas while Im working through this i
side project.  

## 12 Jan 2025

Put together this file and am planning 
to copy any of the mind dump drivle thay 
I write in these files sometimes here to keep it together and out of the source code.  Im not 100% sure what to do about bits that are directly related to a bit of code but figure I cam figure that out later.  

My current thinking here is that Ive really over complicated this thing.  I generally like a couple of things, mainly the measage read and response style a couple of the components have but i dont think its really helping me.

what im going to do is move everything to a systems based approach where each "step" is an individual system thereby hopefully making each component small enough to make it easy to reason about.


## Earlier

/*
@todo:  
need some kind of status 
update to let the additonal threads and 
caller know that we have completed a 
round.  oh also how are we going to 
communicate changes with the calling 
thread.

@note:
Should i change this interface so 
that each thread/module has its own 
"start" function.  ie backend, dealer 
etc.

im starting to think yes, there doesnt 
seem to be a reason not to keep thr DS 
out of the "client" side here, specially 
if each module is going to need to 
access some portion of that structure.

@note2.  
should I have some kind of journal that 
i dump these thoughts knto?  seems like 
a good idea to be honest?
*/